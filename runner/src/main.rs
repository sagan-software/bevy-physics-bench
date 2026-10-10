//! Collect native timings and machine provenance for the shared physics catalog.
//! Command-line input selects a workload, a supported engine matrix, or an
//! optional native viewer. Construction, warmup, pose inspection, and JSON
//! serialization remain outside timed samples. Fresh repetitions alternate
//! engine order and retain every raw sample for independent validation.

use bevy_physics_bench::{
    BodyCount, BodyKind, Dimension, Engine, QueryStatistics, Scenario, SceneFingerprint, Settings,
    SettingsInput, Simulation, Statistics, StepCount, build_metadata,
};
use clap::{Args, Parser, Subcommand};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

#[cfg(feature = "viewer")]
mod viewer;

/// Select a single workload or the complete supported benchmark matrix.
#[derive(Debug, Parser)]
#[command(
    version,
    about = "Collect native Bevy physics timings and machine provenance"
)]
struct Cli {
    /// Requested collection scope.
    #[command(subcommand)]
    command: Command,
}

/// Collection scopes share settings and produce the same JSON result contract.
#[derive(Debug, Subcommand)]
enum Command {
    /// Run a selected scenario with both engines, or only the chosen engine.
    Bench(Options),
    /// Open a native wireframe viewer or export a frame from the shared
    /// simulation.
    #[cfg(feature = "viewer")]
    View(viewer::ViewOptions),
    /// Run every supported scenario in both dimensions and both engines.
    Suite(Options),
}

/// Raw command-line values are validated before any simulation is constructed.
#[derive(Debug, Args)]
struct Options {
    /// Result destination; omit to write JSON to standard output.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Human-readable machine label recorded with the detected hardware.
    #[arg(long, default_value = "local-machine")]
    machine: String,
    /// Limit a single benchmark to one engine.
    #[arg(long)]
    engine: Option<Engine>,
    /// Single-benchmark dimension; suite runs always include both dimensions.
    #[arg(long, default_value = "3d")]
    dimension: Dimension,
    /// Single-benchmark workload; suite runs include the whole catalog.
    #[arg(long, default_value = "pyramid")]
    scenario: Scenario,
    /// Number of scenario bodies, excluding the shared floor.
    #[arg(long, default_value = "128", value_parser = clap::value_parser!(u32).range(1..=32768))]
    count: u32,
    /// Deterministic initial-scene seed.
    #[arg(long, default_value = "42")]
    seed: u32,
    /// Warmup steps excluded from the samples in each fresh repetition.
    #[arg(long, default_value = "60", value_parser = clap::value_parser!(u32).range(1..=600))]
    warmup: u32,
    /// Recorded sample count per fresh repetition.
    #[arg(long, default_value = "30", value_parser = clap::value_parser!(u32).range(5..=1000))]
    samples: u32,
    /// Physics steps timed together and divided into a per-step duration.
    #[arg(long, default_value = "4", value_parser = clap::value_parser!(u32).range(1..=60))]
    batch: u32,
    /// Fresh repetitions, alternating which engine runs first.
    #[arg(long, default_value = "3", value_parser = clap::value_parser!(u32).range(1..=9))]
    repeats: u32,
    /// Optional settings JSON file; scene and dimension still follow
    /// the collection scope.
    #[arg(long)]
    settings: Option<PathBuf>,
}

/// Store typed durations until serialization converts them to milliseconds.
#[derive(Debug)]
struct Collected {
    /// Engine whose world produced these measurements.
    engine: Engine,
    /// Raw per-step samples, excluding warmup.
    samples: Vec<Duration>,
    /// Awake dynamic bodies at the end of the last repetition.
    awake_bodies: usize,
    /// Whether every repetition ended with finite body coordinates.
    is_finite: bool,
    /// Lowest dynamic centre seen at the end of any repetition, in metres.
    min_y: Option<f32>,
    /// Ray statistics from the final measured world, preserving
    /// observed query work.
    queries: QueryStatistics,
}

/// Validate options, collect the requested matrix, and write one result
/// artifact.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the command before performing hardware discovery or simulation
    // work.
    let cli = Cli::parse();
    let (options, scenarios, dimensions) = match cli.command {
        #[cfg(feature = "viewer")]
        Command::View(options) => return viewer::run(options),
        Command::Bench(options) => {
            let scenarios = vec![options.scenario];
            let dimensions = vec![options.dimension];
            (options, scenarios, dimensions)
        }
        Command::Suite(options) => (
            options,
            all_scenarios(),
            vec![Dimension::Two, Dimension::Three],
        ),
    };
    validate_machine(&options.machine)?;
    // Discover host metadata once, outside all construction and timing
    // intervals.
    let system = sysinfo::System::new_all();
    let base = base_settings(&options)?;
    // Collection completes before result encoding or filesystem output begins.
    let measurements = collect_matrix(&options, &scenarios, &dimensions, base)?;
    let result = result_file(&options, &system, measurements)?;
    write_result(&result, options.output.as_ref())
}

/// Recreate worlds per repetition and alternate engine order to reduce
/// order bias.
fn collect(
    engines: &[Engine],
    settings: &Settings,
    options: &Options,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    // Allocate one sample vector per engine before constructing any measured
    // world.
    let mut collected: Vec<Collected> = engines
        .iter()
        .map(|&engine| Collected {
            engine,
            samples: Vec::with_capacity((options.samples * options.repeats) as usize),
            awake_bodies: 0,
            is_finite: true,
            min_y: None,
            queries: QueryStatistics::default(),
        })
        .collect();
    // Reverse engine order between fresh repetitions to reduce consistent order
    // bias.
    for _ in 0..options.repeats {
        for record in &mut collected {
            collect_repeat(record, settings, options)?;
        }
        collected.reverse();
    }
    // Restore the caller's engine order for the exported records.
    if !options.repeats.is_multiple_of(2) {
        collected.reverse();
    }
    collected
        .into_iter()
        .map(|record| encode_measurement(record, settings, options))
        .collect()
}

/// Enumerate every implemented CPU workload for complete matrix collection.
fn all_scenarios() -> Vec<Scenario> {
    vec![
        Scenario::Boxes,
        Scenario::Balls,
        Scenario::Pyramid,
        Scenario::Stacks,
        Scenario::StaticField,
        Scenario::Kinematics,
        Scenario::Dominoes,
        Scenario::JointChain,
        Scenario::Ragdolls,
        Scenario::Ccd,
        Scenario::Queries,
        Scenario::Characters,
        Scenario::InverseKinematics,
    ]
}

/// Collect only combinations supported by the compiled adapters and requested
/// engine filter.
fn collect_matrix(
    options: &Options,
    scenarios: &[Scenario],
    dimensions: &[Dimension],
    base: SettingsInput,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    // Scene and dimension order remain stable across complete-suite
    // collections.
    let mut measurements = Vec::new();
    for &dimension in dimensions {
        for &scenario in scenarios {
            // Revalidate each matrix row after applying its workload and
            // dimension.
            let settings = Settings::try_from(SettingsInput {
                scenario,
                dimension,
                ..base
            })?;
            let engines = selected_engines(scenario, dimension, options.engine);
            // A suite skips unsupported combinations while retaining every
            // supported row.
            if engines.is_empty() {
                continue;
            }
            measurements.extend(collect(&engines, &settings, options)?);
        }
    }
    // An entirely unsupported selection is an error, not a successful empty
    // result.
    if measurements.is_empty() {
        return Err("no supported engine and scene combination was selected".into());
    }
    Ok(measurements)
}

/// Attach detected machine and compiler facts to an already completed set of
/// measurements.
fn result_file(
    options: &Options,
    system: &sysinfo::System,
    measurements: Vec<Value>,
) -> Result<Value, Box<dyn std::error::Error>> {
    // UTC timestamps emit RFC 3339 with exactly millisecond precision and the Z
    // suffix.
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    // Capture actual processor and operating-system facts from this collection
    // host.
    let cpu = system.cpus().first().map(|cpu| cpu.brand());
    let os = sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_owned());
    // Compiled provenance identifies the code and enabled optimization profile.
    let metadata = build_metadata();
    let result = json!({
        "schema_version": 1, "created_at": created_at, "source": "local",
        // Hardware and execution facts delimit comparability across stored
        // runs.
        "environment": { "runtime": "native", "machine": options.machine, "cpu": cpu,
            // Memory is bytes; logical cores describe host capacity, not worker
            // count.
            "logical_cores": system.cpus().len(), "memory_bytes": system.total_memory(),
            "os": os, "browser": null, "target": metadata.get("target"),
            "profile": metadata.get("profile"), "gpu": null,
            // The tuned profile enables worker scheduling; Jolt remains inline.
            "threads": if cfg!(feature = "native-tuned") { std::thread::available_parallelism()?.get() } else { 1 }, "notes": "Bevy integration steps include transform synchronization. Interactive host load is not controlled. Jolt retains its inline job system in both profiles." },
        // Keep exact dependency versions alongside build flags and source
        // identity.
        "versions": metadata.get("versions"), "build": metadata.get("build"),
        "measurements": measurements,
    });
    Ok(result)
}

/// Time one fresh world and retain pose health after all measured steps have
/// completed.
fn collect_repeat(
    record: &mut Collected,
    settings: &Settings,
    options: &Options,
) -> Result<(), Box<dyn std::error::Error>> {
    // Bound both excluded warmup work and the measured synchronous batch.
    let (warmup, batch) = step_counts(options)?;
    let mut simulation = Simulation::new(record.engine, *settings)?;
    simulation.advance(warmup)?;
    // Only the fixed-step call is timed; conversion and storage happen
    // afterward.
    for _ in 0..options.samples {
        let start = Instant::now();
        simulation.advance(batch)?;
        let elapsed = start.elapsed();
        record.samples.push(elapsed / options.batch);
    }
    // Health inspection follows timing and cannot inflate measured step
    // durations.
    let snapshot = simulation.snapshot();
    record.queries = snapshot.queries;
    record.is_finite &= snapshot.bodies.iter().all(|body| {
        body.position
            .iter()
            .chain(&body.rotation)
            .all(|value| value.is_finite())
    });
    record.awake_bodies = snapshot
        .bodies
        .iter()
        .filter(|body| body.kind == BodyKind::Dynamic && !body.is_sleeping)
        .count();
    // Retain the lowest finite dynamic-body centre observed across all
    // repetitions.
    for body in snapshot
        .bodies
        .iter()
        .filter(|body| body.kind == BodyKind::Dynamic)
    {
        let height = body.position[1];
        if height.is_finite() {
            record.min_y = Some(record.min_y.map_or(height, |previous| previous.min(height)));
        }
    }
    Ok(())
}

/// Reject empty labels, oversized labels, and controls before running a
/// benchmark.
fn validate_machine(machine: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Labels are user-visible metadata and cannot contain terminal controls.
    let is_length_valid = (1..=120).contains(&machine.len());
    let has_control = machine.chars().any(char::is_control);
    if !is_length_valid || has_control {
        return Err("machine must contain 1 to 120 bytes without control characters".into());
    }
    Ok(())
}

/// Parse an explicit settings file or construct the documented command
/// defaults.
fn base_settings(options: &Options) -> Result<SettingsInput, Box<dyn std::error::Error>> {
    // Explicit settings files pass through the same validated JSON boundary as
    // the browser.
    if let Some(path) = &options.settings {
        let settings: Settings = std::fs::read_to_string(path)?.parse()?;
        Ok(settings.into())
    } else {
        let defaults = SettingsInput::default();
        Ok(SettingsInput {
            count: BodyCount::try_from(options.count)?,
            seed: options.seed,
            ..defaults
        })
    }
}

/// Write complete JSON after timing has stopped, preserving all raw samples.
fn write_result(result: &Value, path: Option<&PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Encoding happens after measurement and preserves full sample precision.
    let text = serde_json::to_string_pretty(result)?;
    let encoded = format!("{text}\n");
    // File output replaces the explicitly selected destination.
    if let Some(path) = path {
        std::fs::write(path, encoded)?;
    } else {
        // Standard output contains only JSON so it can feed an independent
        // validator.
        std::io::stdout().write_all(encoded.as_bytes())?;
    }
    Ok(())
}

/// Serialize timing and health fields after every repetition has completed.
fn encode_measurement(
    record: Collected,
    settings: &Settings,
    options: &Options,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Summary values and raw samples use milliseconds per fixed simulation
    // step.
    let statistics = Statistics::from_samples(&record.samples)?;
    let samples: Vec<f64> = record
        .samples
        .iter()
        .map(|duration| duration.as_secs_f64() * 1000.0)
        .collect();
    // The plan records warmup exclusions, batching, and independent world
    // count.
    Ok(json!({ "engine": record.engine, "settings": settings,
            "plan": { "warmup_steps": options.warmup, "samples": options.samples, "batch_steps": options.batch, "repeats": options.repeats },
            // Raw per-step samples remain authoritative for recomputing
            // statistics.
            "samples_ms": samples, "statistics": statistics, "awake_bodies": record.awake_bodies,
            // Health summarizes completed poses across repetitions, outside
            // timing.
            "finite": record.is_finite, "min_y": record.min_y, "queries": record.queries,
            // The fingerprint authenticates neither authorship nor evolving
            // trajectories.
            "scene_fingerprint": SceneFingerprint::of(settings)?,
            "scope": if record.engine == Engine::Bevy { "bevy-animation" } else { "bevy-step" }, "readback_ms": null }))
}

/// Intersect runtime capabilities with an optional single-engine selection.
fn selected_engines(
    scenario: Scenario,
    dimension: Dimension,
    selected: Option<Engine>,
) -> Vec<Engine> {
    // Animation has its own series; Jolt is available only for spatial native
    // scenes.
    let mut engines = if scenario == Scenario::InverseKinematics {
        vec![Engine::Bevy]
    } else if dimension == Dimension::Three && cfg!(feature = "jolt") {
        vec![Engine::Avian, Engine::Rapier, Engine::Jolt]
    } else {
        vec![Engine::Avian, Engine::Rapier]
    };
    // Filtering cannot add an engine unsupported by this dimension and
    // workload.
    if let Some(engine) = selected {
        engines.retain(|candidate| *candidate == engine);
    }
    engines
}

/// Convert validated CLI ranges into the public stepping boundary types.
fn step_counts(
    options: &Options,
) -> Result<(StepCount, StepCount), bevy_physics_bench::BenchError> {
    Ok((
        StepCount::try_from(options.warmup)?,
        StepCount::try_from(options.batch)?,
    ))
}

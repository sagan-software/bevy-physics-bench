//! Collect completed Nexus GPU work with separate readback samples.
//!
//! This runner requires an available graphics adapter. It constructs each
//! world before timing, collects fresh repetitions, and preserves the
//! selected device identity. Results share the site schema while retaining
//! a separate GPU measurement scope and profile.

use bevy_physics_bench::{
    BodyCount, BodyKind, Scenario, SceneFingerprint, Settings, SettingsInput, Statistics,
    build_metadata,
};
use bevy_physics_bench_gpu::GpuSimulation;
use clap::{Args, Parser, Subcommand};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

/// Select one scene or every workload in the published GPU capability profile.
#[derive(Debug, Parser)]
struct Cli {
    /// Collection scope and validated command-line options.
    #[command(subcommand)]
    command: Command,
}

/// Keep complete-suite collection distinct from a single configured workload.
#[derive(Debug, Subcommand)]
enum Command {
    /// Measure a selected supported spatial workload.
    Bench(Options),
    /// Measure every supported spatial workload using the same settings.
    Suite(Options),
}

/// Raw collection values are validated before allocating a device or simulation
/// world.
#[derive(Debug, Args)]
struct Options {
    /// Human-readable label recorded beside detected CPU and GPU facts.
    #[arg(long, default_value = "local-gpu")]
    machine: String,
    /// Single-workload identifier; suite collection uses the complete GPU
    /// catalog.
    #[arg(long, default_value = "pyramid")]
    scenario: Scenario,
    /// Number of scene bodies, excluding the shared floor collider.
    #[arg(long, default_value = "128")]
    count: BodyCount,
    /// Optional validated settings JSON, including materials and solver effort.
    #[arg(long)]
    settings: Option<PathBuf>,
    /// Destination JSON file; omit to write the artifact to standard output.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Excluded fixed steps before recording each fresh repetition.
    #[arg(long, default_value = "30", value_parser = clap::value_parser!(u32).range(1..=600))]
    warmup: u32,
    /// Samples collected in each fresh repetition after warmup.
    #[arg(long, default_value = "15", value_parser = clap::value_parser!(u32).range(5..=1000))]
    samples: u32,
    /// Fixed steps per timed sample, divided into milliseconds per step.
    #[arg(long, default_value = "1", value_parser = clap::value_parser!(u32).range(1..=60))]
    batch: u32,
    /// Number of independently constructed worlds measured for each workload.
    #[arg(long, default_value = "3", value_parser = clap::value_parser!(u32).range(1..=9))]
    repeats: u32,
}

/// Parse the closed command vocabulary before entering the asynchronous GPU
/// collector.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the requested collection scope before allocating GPU resources.
    let cli = Cli::parse();
    let (options, scenarios) = match cli.command {
        Command::Bench(options) => {
            let scenario = options.scenario;
            (options, vec![scenario])
        }
        Command::Suite(options) => (
            options,
            vec![
                Scenario::Boxes,
                Scenario::Balls,
                Scenario::Pyramid,
                Scenario::Stacks,
                Scenario::StaticField,
                Scenario::Dominoes,
                Scenario::JointChain,
                Scenario::Ragdolls,
            ],
        ),
    };
    futures::executor::block_on(collect(options, &scenarios))
}

/// Retain raw timings and observed state across fresh worlds of one scene.
struct Collected {
    /// Total completed-step durations, including GPU synchronization.
    samples: Vec<Duration>,
    /// Readback and wait subsets of the total samples in the same order.
    readbacks: Vec<Duration>,
    /// A failed earlier repetition cannot be replaced by a successful later one.
    is_finite: bool,
    /// Lowest finite dynamic-body centre after any measured repetition.
    min_y: Option<f32>,
    /// Hardware identity must remain constant across all repetitions.
    device: Value,
}

/// Collect only scenes accepted by the GPU profile and retain actual device identity.
async fn collect(
    options: Options,
    scenarios: &[Scenario],
) -> Result<(), Box<dyn std::error::Error>> {
    let base = base_settings(&options)?;
    let system = sysinfo::System::new_all();
    let mut measurements = Vec::with_capacity(scenarios.len());
    let mut device = Value::Null;
    // Construct and measure each scene sequentially without sharing evolving state.
    for &scenario in scenarios {
        let settings = Settings::try_from(SettingsInput { scenario, ..base })?;
        let (measurement, metadata) = Box::pin(measure(settings, &options)).await?;
        retain_device(&mut device, metadata)?;
        measurements.push(measurement);
    }
    let result = result_file(&options, &system, device, measurements);
    write_result(&result, options.output.as_ref())
}

/// Validate machine labels and configuration before requesting a device.
fn base_settings(options: &Options) -> Result<SettingsInput, Box<dyn std::error::Error>> {
    // Reject invalid labels before filesystem reads or device selection.
    let is_length_valid = (1..=120).contains(&options.machine.len());
    let has_control = options.machine.chars().any(char::is_control);
    if !is_length_valid || has_control {
        return Err("machine must contain 1 to 120 bytes without control characters".into());
    }
    // Explicit files pass through the same validated settings parser as browser inputs.
    if let Some(path) = &options.settings {
        Ok(std::fs::read_to_string(path)?.parse::<Settings>()?.into())
    } else {
        let defaults = SettingsInput::default();
        Ok(SettingsInput {
            count: options.count,
            ..defaults
        })
    }
}

/// Reject a device change before combining samples into a single hardware series.
fn retain_device(device: &mut Value, incoming: Value) -> Result<(), Box<dyn std::error::Error>> {
    if !device.is_null() && *device != incoming {
        return Err("the selected GPU changed during collection".into());
    }
    *device = incoming;
    Ok(())
}

/// Recreate each scene for independent repetitions and retain all completed samples.
async fn measure(
    settings: Settings,
    options: &Options,
) -> Result<(Value, Value), Box<dyn std::error::Error>> {
    let capacity = (options.samples * options.repeats) as usize;
    let mut collected = Collected {
        samples: Vec::with_capacity(capacity),
        readbacks: Vec::with_capacity(capacity),
        is_finite: true,
        min_y: None,
        device: Value::Null,
    };
    // Pipeline creation and warmup repeat with each new world outside measured samples.
    for _ in 0..options.repeats {
        Box::pin(measure_repeat(&mut collected, settings, options)).await?;
    }
    let result = encode_measurement(&collected, &settings, options)?;
    Ok((result, collected.device))
}

/// Warm one new world and inspect completed poses after the measured batches finish.
async fn measure_repeat(
    collected: &mut Collected,
    settings: Settings,
    options: &Options,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut simulation = GpuSimulation::new(settings).await?;
    retain_device(&mut collected.device, simulation.metadata().clone())?;
    // Warmup includes real completion but contributes no recorded timing samples.
    for _ in 0..options.warmup {
        simulation.advance().await?;
    }
    measure_samples(
        &mut simulation,
        options,
        &mut collected.samples,
        &mut collected.readbacks,
    )
    .await?;
    retain_health(collected, &simulation.snapshot());
    Ok(())
}

/// Retain the worst pose validity and lowest finite height across completed repetitions.
fn retain_health(collected: &mut Collected, snapshot: &bevy_physics_bench::Snapshot) {
    // Pose validation is outside the measured interval and cannot erase earlier failures.
    collected.is_finite &= snapshot.bodies.iter().all(|body| {
        body.position
            .iter()
            .chain(&body.rotation)
            .all(|value| value.is_finite())
    });
    // Heights retain the lowest finite dynamic centre without including static geometry.
    for body in snapshot
        .bodies
        .iter()
        .filter(|body| body.kind == BodyKind::Dynamic)
    {
        let height = body.position[1];
        if height.is_finite() {
            collected.min_y = Some(
                collected
                    .min_y
                    .map_or(height, |previous| previous.min(height)),
            );
        }
    }
}

/// Time completed GPU batches while preserving the readback subset independently.
async fn measure_samples(
    simulation: &mut GpuSimulation,
    options: &Options,
    samples: &mut Vec<Duration>,
    readbacks: &mut Vec<Duration>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Every awaited step includes submission, completion, pose copying, and validation.
    for _ in 0..options.samples {
        let start = Instant::now();
        let mut readback = Duration::ZERO;
        for _ in 0..options.batch {
            simulation.advance().await?;
            readback += simulation.last_readback();
        }
        samples.push(start.elapsed() / options.batch);
        readbacks.push(readback / options.batch);
    }
    Ok(())
}

/// Convert durations to milliseconds per step only at the result serialization boundary.
fn milliseconds(durations: &[Duration]) -> Vec<f64> {
    durations
        .iter()
        .map(|duration| duration.as_secs_f64() * 1000.0)
        .collect()
}

/// Encode raw samples, their summary, and the separate GPU measurement boundary.
fn encode_measurement(
    collected: &Collected,
    settings: &Settings,
    options: &Options,
) -> Result<Value, Box<dyn std::error::Error>> {
    // GPU profile disables sleeping, so every dynamic scene body remains awake.
    let awake = bevy_physics_bench::InitialScene::new(settings)
        .bodies()
        .iter()
        .filter(|body| body.kind() == BodyKind::Dynamic)
        .count();
    let statistics = Statistics::from_samples(&collected.samples)?;
    // The same input fingerprint is shared with compatible CPU scenes.
    let fingerprint = SceneFingerprint::of(settings)?;
    Ok(json!({
        // The engine and scope are fixed by this dedicated GPU collector.
        "engine": "nexus", "settings": settings, "scope": "gpu-step-readback", "scene_fingerprint": fingerprint,
        // The sampling plan identifies excluded warmup and fresh-world repetitions.
        "plan": { "warmup_steps": options.warmup, "samples": options.samples, "batch_steps": options.batch, "repeats": options.repeats },
        // Readback is a subset of total elapsed work, never an additional duration.
        "samples_ms": milliseconds(&collected.samples), "readback_ms": milliseconds(&collected.readbacks), "statistics": statistics,
        // Health describes completed poses; this profile performs no spatial query workload.
        "awake_bodies": awake, "finite": collected.is_finite, "min_y": collected.min_y, "queries": { "rays": 0, "hits": 0 }
    }))
}

/// Attach compiler, host, and selected GPU provenance to completed measurements.
fn result_file(
    options: &Options,
    system: &sysinfo::System,
    device: Value,
    measurements: Vec<Value>,
) -> Value {
    // UTC output uses exactly three fractional digits and the literal Z suffix.
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let metadata = build_metadata();
    // Host facts describe the collector; the GPU identity comes from the actual adapter.
    let cpu = system.cpus().first().map(|cpu| cpu.brand());
    let os = sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_owned());
    json!({
        "schema_version": 1, "created_at": created_at, "source": "local",
        // CPU capacity is separate from this sequential collector's worker count.
        "environment": { "runtime": "native", "machine": options.machine, "cpu": cpu,
            "logical_cores": system.cpus().len(), "memory_bytes": system.total_memory(), "os": os,
            // GPU totals cannot be ranked against CPU-only step scopes.
            "browser": null, "target": metadata.get("target"), "profile": "gpu", "threads": 1, "gpu": device,
            "notes": "GPU totals include submission, completion, pose readback, and capacity validation. Readback includes GPU completion wait. Interactive host load is not controlled." },
        // Dependency and source identities identify the code that produced these samples.
        "versions": metadata.get("versions"),
        "build": { "compiler": metadata.get("build").and_then(|build| build.get("compiler")),
            "rustflags": metadata.get("build").and_then(|build| build.get("rustflags")),
            // Fixed buffer and solver-color policies form part of the experimental profile.
            "features": ["nexus-webgpu", "fixed-pair-capacity", "64-solver-colors"] },
        // Measurement order follows the requested scene catalog without ranking engines.
        "measurements": measurements
    })
}

/// Write one complete JSON artifact after all measurement intervals have ended.
fn write_result(result: &Value, path: Option<&PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Preserve raw sample precision and terminate the artifact with one newline.
    let text = serde_json::to_string_pretty(result)?;
    let encoded = format!("{text}\n");
    // Only an explicitly supplied destination receives filesystem output.
    if let Some(path) = path {
        std::fs::write(path, encoded)?;
    } else {
        // Standard output contains only the machine-readable result document.
        std::io::stdout().write_all(encoded.as_bytes())?;
    }
    Ok(())
}

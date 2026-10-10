//! Validated settings shared across engine adapters.

use self::parameters::{Frequency, Friction, Gravity, Restitution, SceneSeed, SolverEffort};
use crate::{BenchError, BodyCount, Dimension, Scenario};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};

/// Validated physical settings retained after the input boundary, including
/// typed counts, frequency, and material parameters. Serialization derives
/// the documented JSON representation from these values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SettingsInput", into = "SettingsInput")]
pub struct Settings {
    /// Selected workload, shared by all compatible engine adapters.
    scenario: Scenario,
    /// Coordinate space used by both simulation and rendering.
    dimension: Dimension,
    /// Validated number of scenario bodies, excluding the floor.
    count: BodyCount,
    /// Deterministic identity of the scene generator's random sequence.
    seed: SceneSeed,
    /// Closed vocabulary of supported simulation step frequencies.
    frequency: Frequency,
    /// Validated downward acceleration in metres per second squared.
    gravity: Gravity,
    /// Validated surface coefficient used at the engine boundary.
    friction: Friction,
    /// Validated restitution coefficient used at the engine boundary.
    restitution: Restitution,
    /// Validated backend-specific effort, whose units remain engine-specific.
    effort: SolverEffort,
    /// Whether dynamic bodies may use backend sleeping rules.
    sleeping: Sleeping,
    /// Whether dynamic bodies request continuous collision detection.
    ccd: CollisionMode,
}

/// Input fields accepted by the JSON settings boundary before validation
/// creates typed settings. Missing fields receive shared defaults, while
/// unknown fields are rejected.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SettingsInput {
    /// The workload that determines initial geometry, prescribed motion, and
    /// constraints for every compatible adapter. Missing JSON input selects the
    /// default box pile.
    pub scenario: Scenario,
    /// The coordinate dimension used for physics and rendering throughout a
    /// run. Planar scenes use the horizontal and vertical axes with zero depth.
    pub dimension: Dimension,
    /// The validated number of requested scenario bodies, excluding the shared
    /// floor. Partial final chains, ragdolls, or limbs retain exactly this
    /// requested count.
    pub count: BodyCount,
    /// The unsigned seed that initializes the fixed random sequence used for
    /// initial position jitter. Engines receive the same sequence before their
    /// first step.
    pub seed: u32,
    /// Simulation frequency in hertz, accepting only 30, 60, 120, or 240. The
    /// fixed duration is derived once using nanoseconds rounded to the nearest
    /// integer.
    pub hz: u16,
    /// Downward acceleration in metres per second squared, from zero through
    /// fifty inclusive. Nonfinite values are rejected before an engine world is
    /// constructed.
    pub gravity: f32,
    /// The dimensionless surface friction coefficient, from zero through two
    /// inclusive. Adapters assign the same value to every collider and the
    /// shared floor.
    pub friction: f32,
    /// The dimensionless restitution coefficient, from zero through one
    /// inclusive. Adapters assign the same value while retaining their own
    /// contact and restitution algorithms.
    pub restitution: f32,
    /// Backend-specific solver effort from one through thirty-two inclusive.
    /// Avian and Jolt interpret this as substeps, while Rapier uses solver
    /// iterations with different work.
    pub solver_effort: u32,
    /// Whether dynamic bodies can deactivate after settling under the selected
    /// engine's thresholds. Disabled sleeping keeps the default benchmark
    /// workload active throughout its measured steps.
    pub sleeping: Sleeping,
    /// The requested collision sampling policy applied before simulation
    /// begins. Continuous detection enables each supported engine technique
    /// while preserving its own implementation and thresholds.
    pub ccd: CollisionMode,
}

/// A closed policy controlling whether bodies can leave active simulation
/// after settling. Enabling sleeping retains each engine's thresholds and can
/// change measured workload size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sleeping {
    /// Keep every dynamic body active throughout warmup and sampling, including
    /// resting piles. This default avoids measuring mostly sleeping bodies
    /// after the scene settles.
    #[default]
    Disabled,
    /// Allow each supported engine to deactivate bodies using its own sleeping
    /// thresholds. Results record the final awake count because this changes
    /// ongoing simulation work.
    Enabled,
}

/// A closed collision policy selecting discrete or continuous detection
/// where the chosen engine supports it. The policy is part of every scene
/// fingerprint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollisionMode {
    /// Detect collisions at the discrete states processed by the selected
    /// solver. Fast projectiles can cross thin obstacles between states and
    /// expose visible tunneling.
    #[default]
    Discrete,
    /// Request the selected engine's continuous collision detection for moving
    /// bodies. Engines retain their own algorithms, so this option does not
    /// promise identical trajectories.
    Continuous,
}

mod parameters;

impl Default for SettingsInput {
    fn default() -> Self {
        Self {
            scenario: Scenario::default(),
            dimension: Dimension::default(),
            count: BodyCount::default(),
            seed: 42,
            hz: 60,
            gravity: 9.81,
            friction: 0.5,
            restitution: 0.0,
            solver_effort: 4,
            sleeping: Sleeping::default(),
            ccd: CollisionMode::default(),
        }
    }
}

impl TryFrom<SettingsInput> for Settings {
    type Error = BenchError;

    fn try_from(input: SettingsInput) -> Result<Self, Self::Error> {
        // Reject malformed physical values before any engine world is allocated.
        let gravity = Gravity::try_from(input.gravity)?;
        let friction = Friction::try_from(input.friction)?;
        let restitution = Restitution::try_from(input.restitution)?;
        let (frequency, effort) = timing_parameters(&input)?;
        // Retain validated semantic types; the input DTO exists only at the boundary.
        Ok(Self {
            scenario: input.scenario,
            dimension: input.dimension,
            count: input.count,
            seed: SceneSeed::from(input.seed),
            frequency,
            gravity,
            friction,
            restitution,
            effort,
            sleeping: input.sleeping,
            ccd: input.ccd,
        })
    }
}

impl From<Settings> for SettingsInput {
    fn from(value: Settings) -> Self {
        value.input()
    }
}

impl FromStr for Settings {
    type Err = BenchError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(value).map_err(BenchError::Json)
    }
}

impl Settings {
    /// Construct validated default settings for a chosen workload and dimension
    /// without parsing JSON. Other physical parameters, body count, and seed
    /// keep their shared defaults.
    pub fn for_scenario(scenario: Scenario, dimension: Dimension) -> Self {
        Self {
            scenario,
            dimension,
            count: BodyCount::default(),
            seed: SceneSeed::from(42),
            frequency: Frequency::default(),
            gravity: Gravity::default(),
            friction: Friction::default(),
            restitution: Restitution::default(),
            effort: SolverEffort::default(),
            sleeping: Sleeping::default(),
            ccd: CollisionMode::default(),
        }
    }

    /// Copy the validated settings into their JSON boundary representation
    /// without changing the retained domain values. Mutating this copy cannot
    /// alter an existing simulation.
    pub fn input(&self) -> SettingsInput {
        SettingsInput {
            scenario: self.scenario,
            dimension: self.dimension,
            count: self.count,
            seed: self.seed.into(),
            hz: self.frequency.into(),
            gravity: self.gravity.into(),
            friction: self.friction.into(),
            restitution: self.restitution.into(),
            solver_effort: self.effort.into(),
            sleeping: self.sleeping,
            ccd: self.ccd,
        }
    }

    /// Return the fixed simulation duration derived from the accepted frequency
    /// in hertz. The duration uses integer nanoseconds rounded from the
    /// reciprocal frequency.
    pub fn timestep(&self) -> Duration {
        self.frequency.duration()
    }
}

/// Validate the independent frequency and solver-work boundaries.
fn timing_parameters(input: &SettingsInput) -> Result<(Frequency, SolverEffort), BenchError> {
    Ok((
        Frequency::try_from(input.hz)?,
        SolverEffort::try_from(input.solver_effort)?,
    ))
}

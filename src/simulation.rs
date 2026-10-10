//! Owned simulation state and explicit stepping.

use crate::{BenchError, Engine, Settings, Snapshot, StepCount};

/// A complete engine world owned by one caller and advanced through
/// explicit fixed steps. Its retained settings cannot change while a
/// measurement is running.
pub struct Simulation {
    /// Owned Bevy world and engine schedules.
    adapter: crate::adapters::Adapter,
    /// Validated settings used for every tick.
    settings: Settings,
    /// Maximum retained scenario bodies for interactive growth.
    capacity: crate::BodyCount,
    /// Completed fixed steps.
    tick: u64,
}

impl Simulation {
    /// Add a validated velocity change without stepping or replacing the world.
    /// Missing, static, and kinematic targets are rejected before mutation.
    pub fn kick(&mut self, kick: crate::BodyKick) -> Result<(), BenchError> {
        self.adapter.kick(kick)
    }
    /// Construct an engine world from validated settings and shared initial
    /// geometry before timing starts. Unsupported capabilities and backend
    /// construction failures preserve their diagnostic categories.
    pub fn new(engine: Engine, settings: Settings) -> Result<Self, BenchError> {
        Self::with_capacity(engine, settings, settings.input().count)
    }

    /// Reserve a bounded body capacity for growing box or ball workloads.
    /// The capacity includes scenario bodies only and must cover the initial scene.
    pub fn with_capacity(
        engine: Engine,
        settings: Settings,
        capacity: crate::BodyCount,
    ) -> Result<Self, BenchError> {
        // Reject a capacity that cannot hold the requested initial topology.
        let initial_count = settings.input().count;
        if capacity.get() < initial_count.get() {
            return Err(BenchError::InvalidSetting(
                "capacity is smaller than the initial scene",
            ));
        }
        // Generate one immutable input description before constructing the selected backend.
        let scene = crate::InitialScene::with_capacity(&settings, capacity)?;
        let adapter = crate::adapters::Adapter::new(
            engine,
            &settings,
            scene.bodies(),
            capacity,
            scene.floor_half_extents(),
        )?;
        Ok(Self {
            adapter,
            settings,
            capacity,
            tick: 0,
        })
    }

    /// Append a deterministic batch without resetting existing poses or time.
    /// Only boxes and balls support growth; insertion is outside timed step samples.
    pub fn append(&mut self, count: crate::BodyCount) -> Result<(), BenchError> {
        // Check retained capacity before generating or inserting any new body.
        let existing = self.adapter.body_count();
        if existing.get() + count.get() > self.capacity.get() {
            return Err(BenchError::InvalidSetting("body capacity exceeded"));
        }
        let batch = crate::InitialScene::drop_batch(&self.settings, count, existing)?;
        self.adapter.append(&self.settings, batch.bodies())
    }

    /// Advance a validated number of fixed simulation steps without rendering
    /// or serializing poses. Integration failures stop the call before
    /// reporting later steps as completed.
    pub fn advance(&mut self, steps: StepCount) -> Result<(), BenchError> {
        // A failed engine step must not advance the observable completed-step count.
        for _ in 0..steps.get() {
            self.adapter.step(&self.settings)?;
            self.tick += 1;
        }
        Ok(())
    }

    /// Copy completed body poses, geometry, and workload counters for rendering
    /// after timing stops. Body order and passive joint indices remain stable
    /// throughout the simulation.
    pub fn snapshot(&mut self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            bodies: self.adapter.snapshot(),
            joints: self.adapter.joints(),
            queries: self.adapter.queries(),
        }
    }
}

impl std::fmt::Debug for Simulation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Simulation")
            .field("settings", &self.settings)
            .field("tick", &self.tick)
            .finish_non_exhaustive()
    }
}

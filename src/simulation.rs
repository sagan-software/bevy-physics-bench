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
    /// Completed fixed steps.
    tick: u64,
}

impl Simulation {
    /// Construct an engine world from validated settings and shared initial
    /// geometry before timing starts. Unsupported capabilities and backend
    /// construction failures preserve their diagnostic categories.
    pub fn new(engine: Engine, settings: Settings) -> Result<Self, BenchError> {
        // Generate one immutable input description before constructing the selected backend.
        let bodies = crate::scene::generate(&settings);
        let adapter = crate::adapters::Adapter::new(engine, &settings, &bodies)?;
        Ok(Self {
            adapter,
            settings,
            tick: 0,
        })
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

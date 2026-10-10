//! Run the same falling-body scene through both Bevy physics integrations.

use bevy_physics_bench::{Engine, Settings, Simulation, StepCount};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for engine in [Engine::Avian, Engine::Rapier] {
        let mut simulation = Simulation::new(engine, Settings::default())?;
        simulation.advance(StepCount::try_from(60)?)?;
        let snapshot = simulation.snapshot();
        println!("{engine:?}: {snapshot:?}");
    }
    Ok(())
}

//! Run a shared scene on an available GPU and wait for completed poses.

use bevy_physics_bench::Settings;
use bevy_physics_bench_gpu::GpuSimulation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    futures::executor::block_on(async {
        let mut simulation = GpuSimulation::new(Settings::default()).await?;
        simulation.advance().await?;
        let snapshot = simulation.snapshot();
        println!("{snapshot:?}");
        Ok(())
    })
}

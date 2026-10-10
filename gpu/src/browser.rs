//! Asynchronous browser ownership and measurement access for the GPU adapter.

use crate::GpuSimulation;
use wasm_bindgen::prelude::*;

/// Own one WebGPU simulation whose steps complete asynchronously before
/// poses become visible.
/// JavaScript releases the wrapper only after its pending operation has
/// finished.
#[wasm_bindgen]
pub struct BrowserGpu {
    /// GPU resources and the most recently completed pose readback.
    simulation: GpuSimulation,
    /// Sum of wait and readback durations across the previous requested batch.
    readback: std::time::Duration,
}

#[wasm_bindgen]
impl BrowserGpu {
    /// Allocate a growing WebGPU world with an explicit scenario-body capacity.
    /// Device limits are checked against that capacity before allocating buffers.
    pub async fn growing(settings: &str, capacity: u32) -> Result<BrowserGpu, JsValue> {
        let settings = settings.parse().map_err(js_error)?;
        let capacity = bevy_physics_bench::BodyCount::try_from(capacity).map_err(js_error)?;
        let simulation = GpuSimulation::with_capacity(settings, capacity)
            .await
            .map_err(js_error)?;
        Ok(Self {
            simulation,
            readback: std::time::Duration::ZERO,
        })
    }

    /// Add a world-space velocity change in m/s after the preceding step completes.
    /// The promise resolves after the original GPU velocity has been read.
    pub async fn kick(&mut self, index: usize, x: f32, y: f32, z: f32) -> Result<(), JsValue> {
        let kick = bevy_physics_bench::BodyKick::try_from((index, [x, y, z])).map_err(js_error)?;
        self.simulation.kick(kick).await.map_err(js_error)
    }

    /// Append a primitive batch without replacing the owned GPU world.
    /// A capacity error rejects the operation before any insertion.
    pub async fn append(&mut self, count: u32) -> Result<(), JsValue> {
        let count = bevy_physics_bench::BodyCount::try_from(count).map_err(js_error)?;
        self.simulation.append(count).await.map_err(js_error)
    }
    /// Validate settings and request an available WebGPU device before
    /// exposing a simulation.
    /// Device and shader failures reject the promise with their
    /// diagnostic text.
    pub async fn create(settings: &str) -> Result<BrowserGpu, JsValue> {
        console_error_panic_hook::set_once();
        let settings = settings.parse().map_err(js_error)?;
        let simulation = GpuSimulation::new(settings).await.map_err(js_error)?;
        Ok(Self {
            simulation,
            readback: std::time::Duration::ZERO,
        })
    }
    /// Complete a bounded batch of steps, accumulating readback
    /// durations inside the measured operation.
    /// The returned promise resolves only after every requested step
    /// has completed.
    pub async fn advance(&mut self, steps: u32) -> Result<(), JsValue> {
        let steps = bevy_physics_bench::StepCount::try_from(steps).map_err(js_error)?;
        self.readback = std::time::Duration::ZERO;
        for _ in 0..steps.get() {
            self.simulation.advance().await.map_err(js_error)?;
            self.readback += self.simulation.last_readback();
        }
        Ok(())
    }
    /// Serialize completed poses after the asynchronous step has
    /// resolved and timing has stopped.
    /// Geometry and body order follow the same snapshot contract as CPU
    /// adapters.
    pub fn snapshot(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.simulation.snapshot()).map_err(js_error)
    }
    /// Return the shared identity of initial geometry and settings
    /// before any GPU simulation steps.
    /// This digest excludes evolving poses and remains constant across batches.
    pub fn fingerprint(&self) -> String {
        self.simulation.fingerprint().to_string()
    }
    /// Read the device metadata from the adapter actually selected for
    /// this owned GPU simulation.
    /// The browser collector includes these facts in exported benchmark
    /// records.
    pub fn metadata(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.simulation.metadata()).map_err(js_error)
    }
    /// Return the previous batch's GPU wait and readback time in
    /// milliseconds before per-step division.
    /// This duration is included in the total elapsed sample.
    pub fn readback_ms(&self) -> f64 {
        self.readback.as_secs_f64() * 1000.0
    }
}

/// Convert a diagnostic into a JavaScript exception only at the browser
/// boundary.
fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

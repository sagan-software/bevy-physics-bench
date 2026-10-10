//! WebAssembly boundary for the static browser application.

use crate::{Simulation, StepCount};
use wasm_bindgen::prelude::*;

/// A real Bevy simulation owned by JavaScript and released with `free()`.
/// Rendering and measurement orchestration remain outside this object's
/// fixed-step physics schedule.
#[wasm_bindgen]
pub struct BrowserSimulation {
    /// The Rust simulation and its owned engine world.
    simulation: Simulation,
    /// Identity of the validated, generated initial scene before any steps.
    fingerprint: crate::SceneFingerprint,
}

#[wasm_bindgen]
impl BrowserSimulation {
    /// Construct a growing world with an explicit maximum scenario-body count.
    /// The normal constructor retains its original fixed-size allocation policy.
    pub fn growing(engine: &str, settings: &str, capacity: u32) -> Result<Self, JsValue> {
        let engine = engine.parse().map_err(js_error)?;
        let settings = settings.parse().map_err(js_error)?;
        let fingerprint = crate::SceneFingerprint::of(&settings).map_err(js_error)?;
        let simulation = Simulation::with_capacity(
            engine,
            settings,
            crate::BodyCount::try_from(capacity).map_err(js_error)?,
        )
        .map_err(js_error)?;
        Ok(Self {
            simulation,
            fingerprint,
        })
    }

    /// Append validated boxes or balls while retaining every existing body.
    /// A capacity or unsupported-scene error requires no simulation step.
    pub fn append(&mut self, count: u32) -> Result<(), JsValue> {
        self.simulation
            .append(crate::BodyCount::try_from(count).map_err(js_error)?)
            .map_err(js_error)
    }
    /// Kick one dynamic body using a world-space velocity change in m/s.
    /// Validation completes before the owned world is mutated.
    pub fn kick(&mut self, index: usize, x: f32, y: f32, z: f32) -> Result<(), JsValue> {
        self.simulation
            .kick(crate::BodyKick::try_from((index, [x, y, z])).map_err(js_error)?)
            .map_err(js_error)
    }
    /// Validate an engine identifier and JSON settings before
    /// constructing the world.
    /// Invalid input returns a JavaScript error without retaining a
    /// partial simulation.
    #[wasm_bindgen(constructor)]
    pub fn new(engine: &str, settings: &str) -> Result<Self, JsValue> {
        console_error_panic_hook::set_once();
        let engine = engine.parse().map_err(js_error)?;
        let settings = settings.parse().map_err(js_error)?;
        let fingerprint = crate::SceneFingerprint::of(&settings).map_err(js_error)?;
        let simulation = Simulation::new(engine, settings).map_err(js_error)?;
        Ok(Self {
            simulation,
            fingerprint,
        })
    }

    /// Advance a bounded number of fixed steps without serializing
    /// poses or rendering.
    /// Browser measurement uses elapsed `performance.now()` around this
    /// call, divided by steps.
    pub fn advance(&mut self, steps: u32) -> Result<(), JsValue> {
        self.simulation
            .advance(StepCount::try_from(steps).map_err(js_error)?)
            .map_err(js_error)
    }

    /// Serialize the current poses after stepping, outside the timed
    /// measurement interval.
    /// The returned JSON contains stable body order, geometry, sleeping
    /// state, and joint links.
    pub fn snapshot(&mut self) -> Result<String, JsValue> {
        serde_json::to_string(&self.simulation.snapshot()).map_err(js_error)
    }

    /// Return the initial-scene identity used to verify compatible
    /// benchmark inputs.
    /// The digest excludes evolving poses and never changes when the
    /// simulation advances.
    pub fn fingerprint(&self) -> String {
        self.fingerprint.to_string()
    }
}

/// Preserve the diagnostic text at the JavaScript exception boundary.
fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Export the same generated geometry, velocities, and joint anchors for
/// external adapters.
/// Scene construction and this JSON transfer happen before warmup or timing
/// begins.
#[wasm_bindgen]
pub fn describe_scene(settings: &str) -> Result<String, JsValue> {
    let settings = settings.parse().map_err(js_error)?;
    let mut scene = crate::scene::describe(&settings);
    scene["fingerprint"] =
        serde_json::json!(crate::SceneFingerprint::of(&settings).map_err(js_error)?);
    serde_json::to_string(&scene).map_err(js_error)
}

/// Export a growing batch using the exact geometry consumed by Rust adapters.
/// Stable indices define positions independently of the engine's evolving poses.
#[wasm_bindgen]
pub fn describe_drop(settings: &str, count: u32, existing: u32) -> Result<String, JsValue> {
    let settings = settings.parse().map_err(js_error)?;
    let scene = crate::InitialScene::drop_batch(
        &settings,
        crate::BodyCount::try_from(count).map_err(js_error)?,
        crate::BodyCount::try_from(existing).map_err(js_error)?,
    )
    .map_err(js_error)?;
    serde_json::to_string(&crate::scene::encode_bodies(scene.bodies())).map_err(js_error)
}

/// Export facts from the exact WebAssembly module executing the benchmark's
/// physics steps.
/// The site records this metadata with every result instead of inferring a
/// compiler profile.
#[wasm_bindgen]
pub fn build_metadata() -> Result<String, JsValue> {
    serde_json::to_string(&crate::build_metadata()).map_err(js_error)
}

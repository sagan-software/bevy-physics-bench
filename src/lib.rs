//! Reproducible Bevy physics scenes shared by native and WebAssembly
//! runners. Validated settings determine initial geometry, materials, and
//! fixed simulation steps. Each simulation owns its engine world. Render
//! snapshots and summary statistics remain separate from the timed
//! operation. Engine adapters preserve their distinct solver and controller
//! behavior.

mod adapters;
mod fingerprint;
mod model;
mod provenance;
mod scene;
mod simulation;
mod snapshot;
mod statistics;

pub use self::fingerprint::SceneFingerprint;
pub use self::model::body_count::BodyCount;
pub use self::model::body_kick::BodyKick;
pub use self::model::body_kind::BodyKind;
pub use self::model::dimension::Dimension;
pub use self::model::engine::Engine;
pub use self::model::error::BenchError;
pub use self::model::scenario::Scenario;
pub use self::model::settings::{CollisionMode, Settings, SettingsInput, Sleeping};
pub use self::provenance::build_metadata;
pub use self::scene::{InitialBody, InitialJoint, InitialScene, Shape};
pub use self::statistics::Statistics;

#[cfg(target_arch = "wasm32")]
mod browser;
pub use self::model::step_count::StepCount;
pub use self::simulation::Simulation;
pub use self::snapshot::{BodySnapshot, QueryStatistics, Snapshot};

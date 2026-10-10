//! Experimental GPU simulations share validated benchmark scenes with the
//! CPU adapters.
//!
//! Construction validates capabilities before requesting a graphics device.
//! Each step waits
//! for completed poses, so elapsed time includes submission, GPU work,
//! synchronization,
//! and readback. Rendering and JSON serialization remain outside that interval.

mod device;
mod error;
mod gpu_simulation;
mod scene;

pub use self::gpu_simulation::GpuSimulation;

#[cfg(target_arch = "wasm32")]
mod browser;

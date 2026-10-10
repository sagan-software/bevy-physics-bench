//! Error categories at the simulation boundary.

/// Failures exposed by configuration parsing, capability validation, backend
/// construction, and simulation. Diagnostic source errors remain attached where
/// an underlying parser or engine supplies one.
#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    /// The requested settings are structurally valid but the chosen runtime or
    /// engine cannot execute that workload. The message identifies the rejected
    /// capability or profile requirement.
    #[error("Unsupported configuration: {0}")]
    Unsupported(&'static str),
    /// The engine failed while constructing or advancing its world, including
    /// capacity and synchronization failures. The original error remains
    /// available through the standard error source chain.
    #[error("Engine integration failed: {0}")]
    Engine(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// A supplied value lies outside the documented configuration profile,
    /// including count and material bounds. Validation rejects it before a
    /// corresponding simulation world can be constructed.
    #[error("Invalid setting: {0}")]
    InvalidSetting(&'static str),
    /// JSON could not be decoded into validated settings or serialized at an
    /// output boundary. The original JSON error preserves its location and
    /// underlying diagnostic information.
    #[error("Invalid settings JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// A textual value does not name a supported engine, dimension, or scenario
    /// in its closed vocabulary. The deserializer error preserves the rejected
    /// value and expected alternatives.
    #[error("Invalid value: {0}")]
    Vocabulary(#[from] serde::de::value::Error),
    /// A textual body count cannot be parsed as an unsigned integer
    /// representable by the input type. The original integer parser error
    /// distinguishes malformed and overflowing input.
    #[error("Invalid body count: {0}")]
    Count(#[from] std::num::ParseIntError),
    /// An engine produced a body pose containing a nonfinite component after
    /// completing a requested step. The caller must reject that measurement and
    /// reset the simulation.
    #[error("The simulation produced a nonfinite body state; reset or reduce the timestep")]
    NumericalFailure,
}

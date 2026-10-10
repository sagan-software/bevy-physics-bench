//! Preserve backend failures in the shared benchmark error source chain.

/// Retain the original device or engine error for diagnostics.
pub(crate) fn failure(
    error: impl std::error::Error + Send + Sync + 'static,
) -> bevy_physics_bench::BenchError {
    bevy_physics_bench::BenchError::Engine(Box::new(error))
}

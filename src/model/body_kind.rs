//! Body motion categories used by the scene model and renderer.

/// Motion ownership shared by the initial scene and render snapshots. Each
/// category determines whether the backend integrates forces, fixes geometry,
/// or applies prescribed motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyKind {
    /// Immobile collision geometry that participates in contacts without
    /// integrating forces or prescribed velocity. Static bodies remain part of
    /// the requested scene count and rendered output.
    Static,
    /// A body whose position and velocity change through forces, gravity,
    /// constraints, and collision response. Its sleeping state can reduce
    /// active work when the policy permits.
    Dynamic,
    /// A body moved by a prescribed platform velocity or a character
    /// controller. The workload supplies its motion instead of allowing gravity
    /// to integrate it directly.
    Kinematic,
}

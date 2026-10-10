//! Renderable snapshots outside the timed physics step.

/// A complete renderable state after an integer number of completed
/// simulation steps. The owned snapshot remains independent of later
/// changes to the engine world.
#[derive(Debug, PartialEq, serde::Serialize)]
pub struct Snapshot {
    /// The number of fully completed fixed simulation steps since construction
    /// of the world. Warmup steps are included because they advance the same
    /// physical state.
    pub tick: u64,
    /// Body poses and primitive geometry in the shared scene creation order,
    /// excluding the floor. Joint indices refer to this same ordered body
    /// collection.
    pub bodies: Vec<BodySnapshot>,
    /// Pairs of indices into the body collection connected by passive joints or
    /// animation links. The renderer uses these pairs outside the timed
    /// simulation step.
    pub joints: Vec<[usize; 2]>,
    /// The number of rays and reported hits from the last completed query
    /// workload step. Other workloads leave both counters at their initial zero
    /// values.
    pub queries: QueryStatistics,
}

/// One body pose and primitive geometry copied from a completed engine
/// step. Coordinates use metres and remain in the shared right-handed scene
/// convention.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct BodySnapshot {
    /// The body centre in Cartesian metres, ordered along the horizontal,
    /// vertical, and depth axes. Positive vertical coordinates point upward
    /// from the floor surface.
    pub position: [f32; 3],
    /// The body orientation as a unit quaternion in XYZW order. Planar physics
    /// changes only rotation around the depth axis while preserving this common
    /// representation.
    pub rotation: [f32; 4],
    /// The unchanged primitive geometry used to create this body before
    /// simulation began. Boxes retain their half extents and balls retain their
    /// radius in metres.
    pub shape: crate::scene::Shape,
    /// Whether the selected engine reports this body as sleeping after the
    /// completed step. Collectors count only awake dynamic bodies when
    /// reporting active workload size.
    #[serde(rename = "sleeping")]
    pub is_sleeping: bool,
    /// The original motion category assigned by the shared scene generator
    /// before engine construction. It distinguishes fixed colliders, simulated
    /// bodies, and prescribed kinematic motion.
    pub kind: crate::BodyKind,
}

/// Observable counters for ray queries performed during one completed
/// simulation step. Short rays alternate expected body hits with displaced
/// misses that cannot reach the floor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, bevy::prelude::Resource)]
pub struct QueryStatistics {
    /// The total number of rays cast during the most recent query workload
    /// step. A query scene casts exactly one ray per requested body.
    pub rays: usize,
    /// The number of rays whose selected engine query reported a collider
    /// within the finite ray range. This is observed work rather than an
    /// expected count.
    pub hits: usize,
}

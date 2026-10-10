//! Collision geometry exposed to renderers and external engine adapters.

/// Primitive collision geometry shared by scene generation, external engine
/// adapters, and rendering. Dimensions use metres and remain unchanged when
/// snapshots update body poses after simulation.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Shape {
    /// A rectangular box described by three half extents in metres along its
    /// local axes. Planar adapters use the first two components and retain the
    /// same render geometry.
    Box {
        /// Half width, half height, and half depth in metres, ordered along
        /// local X, Y, and Z axes. Rotation is stored separately in the
        /// containing body pose.
        half_extents: [f32; 3],
    },
    /// A round primitive represented as a circle by planar adapters and a
    /// sphere by spatial adapters. Its common radius determines both collision
    /// and rendered geometry.
    Ball {
        /// Radius in metres measured from the local origin to the primitive
        /// surface. The same positive value is supplied to the backend collider
        /// and the renderer.
        radius: f32,
    },
}

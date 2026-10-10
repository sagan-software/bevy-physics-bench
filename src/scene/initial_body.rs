//! Read-only initial state for one generated scenario body.

use crate::{BodyKind, Shape};
use bevy::prelude::*;

/// A primitive initial pose, motion category, velocity, and optional
/// passive constraint before any engine constructs its world. Public
/// accessors expose this immutable shared description.
#[derive(Clone, Copy, Debug)]
pub struct InitialBody {
    /// Centre in metres, with Y upward.
    pub(crate) position: Vec3,
    /// Initial orientation.
    pub(crate) rotation: Quat,
    /// Collision and render geometry.
    pub(crate) shape: Shape,
    /// Motion category, independent of backend vocabulary.
    pub(crate) kind: BodyKind,
    /// Initial linear velocity in metres per second.
    pub(crate) velocity: Vec3,
    /// Initial angular velocity in radians per second.
    pub(crate) angular_velocity: Vec3,
    /// Optional point constraint to an earlier body in the scene.
    pub(crate) joint: Option<super::InitialJoint>,
}

impl InitialBody {
    /// Read the body's initial centre in metres using Cartesian
    /// coordinates with positive Y upward.
    /// Every adapter receives this same generated position.
    pub fn position(&self) -> Vec3 {
        self.position
    }
    /// Read the initial unit quaternion shared by all compatible
    /// physics adapters.
    /// The rotation follows Bevy's right-handed coordinate conventions
    /// in three dimensions.
    pub fn rotation(&self) -> Quat {
        self.rotation
    }
    /// Read the generated primitive geometry without changing its size
    /// or ownership.
    /// Box dimensions are half extents and ball dimensions are radii in metres.
    pub fn shape(&self) -> Shape {
        self.shape
    }
    /// Read whether this body is static, dynamic, or kinematic in the
    /// shared scene.
    /// Adapters translate that identity into their backend's motion type.
    pub fn kind(&self) -> BodyKind {
        self.kind
    }
    /// Read initial linear velocity in metres per second along the
    /// shared coordinate axes.
    /// This value is assigned before the first warmup step.
    pub fn linear_velocity(&self) -> Vec3 {
        self.velocity
    }
    /// Read initial angular velocity in radians per second around each
    /// coordinate axis.
    /// Planar adapters consume only the component around the Z axis.
    pub fn angular_velocity(&self) -> Vec3 {
        self.angular_velocity
    }
    /// Read the optional passive constraint joining this body to an
    /// earlier scene body.
    /// The returned anchors remain relative to the bodies' local frames.
    pub fn joint(&self) -> Option<super::InitialJoint> {
        self.joint
    }
}

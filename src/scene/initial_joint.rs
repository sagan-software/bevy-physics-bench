//! Shared passive constraint anchors for external physics adapters.

use bevy::prelude::*;

/// A passive point constraint joining an earlier parent body to this child
/// body. Both anchors use local primitive coordinates measured from each
/// body centre.
#[derive(Clone, Copy, Debug)]
pub struct InitialJoint {
    /// Index of the already-created parent body.
    pub(crate) parent: usize,
    /// Parent-local anchor in metres.
    pub(crate) parent_anchor: Vec3,
    /// Child-local anchor in metres.
    pub(crate) child_anchor: Vec3,
}

impl InitialJoint {
    /// Return the parent's zero-based index in the shared initial body slice.
    /// Generation guarantees that this index precedes the constrained
    /// child body.
    pub fn parent(self) -> usize {
        self.parent
    }
    /// Read the parent-local anchor position in metres relative to its
    /// primitive centre.
    /// Adapters use this anchor without applying a world-space translation.
    pub fn parent_anchor(self) -> Vec3 {
        self.parent_anchor
    }
    /// Read the child-local anchor position in metres relative to its
    /// primitive centre.
    /// The constraint joins this point to the parent-local anchor.
    pub fn child_anchor(self) -> Vec3 {
        self.child_anchor
    }
}

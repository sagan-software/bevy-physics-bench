//! Owned initial geometry shared by native and browser engine adapters.

use super::InitialBody;
use crate::Settings;
use bevy::prelude::*;

/// Read-only generated bodies and floor dimensions for one validated scene
/// configuration.
/// Construction performs no simulation and borrows no external engine
/// resource or rendering state.
#[derive(Debug)]
pub struct InitialScene {
    /// Bodies in stable creation order, excluding the common floor.
    bodies: Vec<InitialBody>,
    /// Floor half extents in metres, with its top face at zero height.
    floor: Vec3,
}

impl InitialScene {
    /// Generate an engine-independent scene from validated settings
    /// before simulation or timing begins.
    /// The same settings produce identical initial geometry for every
    /// compatible adapter.
    pub fn new(settings: &Settings) -> Self {
        let bodies = super::generate(settings);
        let floor = super::floor_half_extents(&bodies, settings);
        Self { bodies, floor }
    }
    /// Borrow the generated bodies in their stable creation order
    /// without allocating another collection.
    /// Joint parent indices refer to this same slice.
    pub fn bodies(&self) -> &[InitialBody] {
        &self.bodies
    }
    /// Read the floor's half width, half thickness, and half depth in metres.
    /// Its centre lies one half thickness below zero height.
    pub fn floor_half_extents(&self) -> Vec3 {
        self.floor
    }
}

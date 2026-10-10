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
    /// Generate the next box or ball batch on a fixed eight-wide lattice.
    /// Stable indices determine positions in metres, independent of engine poses.
    pub fn drop_batch(
        settings: &Settings,
        count: crate::BodyCount,
        existing: crate::BodyCount,
    ) -> Result<Self, crate::BenchError> {
        // Restrict growth to unjointed primitives before allocating any batch.
        let input = settings.input();
        if !matches!(
            input.scenario,
            crate::Scenario::Boxes | crate::Scenario::Balls
        ) {
            return Err(crate::BenchError::Unsupported(
                "growth requires boxes or balls",
            ));
        }
        crate::BodyCount::try_from(existing.get() + count.get())?;
        // Eight columns, spaced 1.1 m apart, fit within every shared pile floor.
        let depth = if input.dimension == crate::Dimension::Two {
            1
        } else {
            8
        };
        let bodies = (existing.get()..existing.get() + count.get())
            .map(|index| {
                let position = Vec3::new(
                    (index % 8) as f32 * 1.1 - 3.85,
                    10.0 + (index / (8 * depth)) as f32 * 1.1,
                    ((index / 8) % depth) as f32 * 1.1 - (depth - 1) as f32 * 0.55,
                );
                super::primitive(position, input.scenario)
            })
            .collect();
        Ok(Self {
            bodies,
            floor: Vec3::new(20.0, 0.5, 20.0),
        })
    }
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

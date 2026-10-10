//! Seeded, engine-independent body and joint descriptions.

use crate::{BodyKind, Dimension, Scenario, Settings};
use bevy::prelude::*;

mod initial_body;
mod initial_joint;
mod initial_scene;
mod shape;
mod workloads;

pub(crate) use self::{InitialBody as Body, InitialJoint as Joint};
pub use self::{
    initial_body::InitialBody, initial_joint::InitialJoint, initial_scene::InitialScene,
    shape::Shape,
};

/// Build the configured scene in stable body order.
pub(crate) fn generate(settings: &Settings) -> Vec<Body> {
    // Derive lattice dimensions from the exact requested count.
    let input = settings.input();
    let count = input.count.get();
    let width = lattice_width(count, input.dimension);
    let depth = if input.dimension == Dimension::Two {
        1
    } else {
        width
    };
    // Retain only bounded generator state and preallocate the output body list.
    let mut random = input.seed;
    let mut bodies = Vec::with_capacity(count as usize);
    // Find the smallest triangular base that can contain every requested box.
    let mut pyramid_base = 1;
    while pyramid_base * (pyramid_base + 1) / 2 < count {
        pyramid_base += 1;
    }
    // Row and column advance in creation order, including partial final rows.
    let mut row = 0;
    let mut column = 0;
    for index in 0..count {
        // A fixed LCG gives the same initial jitter on every target and engine.
        random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let jitter = ((random >> 16) as f32 / 65535.0 - 0.5) * 0.04;
        let position = if input.scenario == Scenario::Pyramid {
            pyramid_position(pyramid_base, &mut row, &mut column)
        } else {
            Vec3::new(
                (index % width) as f32 * 1.1 - (width - 1) as f32 * 0.55 + jitter,
                (index / (width * depth)) as f32 * 1.1 + 2.0,
                ((index / width) % depth) as f32 * 1.1 - (depth - 1) as f32 * 0.55,
            )
        };
        let mut body = primitive(position, input.scenario);
        // Each workload changes only its documented geometry and motion.
        workloads::configure(&mut body, index, width, &input);
        bodies.push(body);
    }
    bodies
}

/// Derive the finite floor from scene bounds, retaining extra travel room
/// for projectiles.
pub(crate) fn floor_half_extents(bodies: &[Body], settings: &Settings) -> Vec3 {
    // Projectile scenes reserve additional horizontal travel around the initial bounds.
    let is_ccd = settings.input().scenario == Scenario::Ccd;
    let margin = if is_ccd { 100.0 } else { 10.0 };
    let extent = bodies
        .iter()
        .map(|body| body.position.abs().max_element())
        .fold(10.0_f32, f32::max)
        + margin;
    // The CCD floor is five centimetres thick; other floors are one metre thick.
    let half_thickness = if is_ccd { 0.025 } else { 0.5 };
    Vec3::new(extent, half_thickness, extent)
}

/// Serialize the shared initial scene for adapters outside the Rust module.
pub(crate) fn describe(settings: &Settings) -> serde_json::Value {
    // Encode the same geometry consumed by native adapters before any simulation.
    let bodies = generate(settings);
    let floor = floor_half_extents(&bodies, settings);
    // Stable body order also defines every serialized joint parent index.
    let encoded: Vec<_> = bodies.iter().map(|body| serde_json::json!({
        "position": body.position.to_array(), "rotation": body.rotation.to_array(),
        "shape": body.shape, "kind": body.kind,
        "velocity": body.velocity.to_array(), "angular_velocity": body.angular_velocity.to_array(),
        "joint": body.joint.map(|joint| serde_json::json!({ "parent": joint.parent,
            "parent_anchor": joint.parent_anchor.to_array(), "child_anchor": joint.child_anchor.to_array() })),
    })).collect();
    serde_json::json!({ "bodies": encoded, "floor_half_extents": floor.to_array() })
}

/// Derive an integer lattice ceiling without target-dependent floating root
/// rounding.
fn lattice_width(count: u32, dimension: Dimension) -> u32 {
    // Integer ceilings preserve the same lattice size across native and WebAssembly targets.
    let exponent = match dimension {
        Dimension::Two => 2,
        Dimension::Three => 3,
    };
    // The smallest sufficient width prevents target-dependent rounding at perfect powers.
    let mut width = 1_u32;
    while width.pow(exponent) < count {
        width += 1;
    }
    width
}

/// Advance one triangular row while preserving stable left-to-right body
/// creation order.
fn pyramid_position(base: u32, row: &mut u32, column: &mut u32) -> Vec3 {
    // Boxes are one metre wide and centred horizontally within their row.
    let row_width = base - *row;
    let position = Vec3::new(
        *column as f32 - (row_width - 1) as f32 * 0.5,
        *row as f32 + 0.5,
        0.0,
    );
    // Advance to the next narrower row only after completing the current row.
    *column += 1;
    if *column == row_width {
        *row += 1;
        *column = 0;
    }
    position
}

/// Initialize shared primitive state before applying any workload-specific
/// geometry or motion.
fn primitive(position: Vec3, scenario: Scenario) -> Body {
    let shape = if scenario == Scenario::Balls {
        Shape::Ball { radius: 0.5 }
    } else {
        Shape::Box {
            half_extents: [0.5; 3],
        }
    };
    Body {
        position,
        rotation: Quat::IDENTITY,
        shape,
        kind: BodyKind::Dynamic,
        velocity: Vec3::ZERO,
        angular_velocity: Vec3::ZERO,
        joint: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_boxes_do_not_overlap() {
        let bodies = generate(&Settings::default());
        for (index, body) in bodies.iter().enumerate() {
            for other in bodies.iter().skip(index + 1) {
                assert!((body.position - other.position).abs().max_element() >= 1.0);
            }
        }
    }
}

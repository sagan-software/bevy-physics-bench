//! Workload-specific geometry and motion derived from validated settings.

use super::{Body, Joint, Shape};
use crate::{BodyKind, Dimension, Scenario, SettingsInput};
use bevy::prelude::*;

/// Apply one closed workload profile to the primitive's shared initial state.
pub(super) fn configure(body: &mut Body, index: u32, width: u32, input: &SettingsInput) {
    // Dispatch geometry profiles without letting one scene inherit another mutation.
    match input.scenario {
        Scenario::Stacks => stack(body, index),
        Scenario::Kinematics => platform(body, index),
        Scenario::Dominoes => domino(body, index, input.count.get()),
        Scenario::JointChain => chain(body, index),
        Scenario::Ragdolls => configure_ragdoll(body, index, width, input.dimension),
        Scenario::Ccd => projectile(body),
        Scenario::Characters => character(body, index),
        Scenario::InverseKinematics => inverse_kinematics(body, index),
        // Primitive scenes only alter motion ownership, leaving geometry unchanged.
        Scenario::Boxes
        | Scenario::Balls
        | Scenario::Pyramid
        | Scenario::StaticField
        | Scenario::Queries => primitive_motion(body, index, input.scenario),
    }
}

/// Configure the shared stack geometry and motion before any engine creates
/// bodies.
fn stack(body: &mut Body, index: u32) {
    body.position = Vec3::new(
        (index / 12) as f32 * 2.0,
        (index % 12) as f32 * 1.01 + 0.5,
        0.0,
    )
}

/// Configure the shared platform geometry and motion before any engine creates
/// bodies.
fn platform(body: &mut Body, index: u32) {
    if index.is_multiple_of(8) {
        body.kind = BodyKind::Kinematic;
        body.position.y = 0.5;
        body.shape = Shape::Box {
            half_extents: [1.5, 0.25, 1.5],
        };
    }
}

/// Configure the shared domino geometry and motion before any engine creates
/// bodies.
fn domino(body: &mut Body, index: u32, count: u32) {
    body.position = Vec3::new(index as f32 * 0.55 - count as f32 * 0.275, 0.6, 0.0);
    body.shape = Shape::Box {
        half_extents: [0.09, 0.6, 0.3],
    };
    if index == 0 {
        body.angular_velocity.z = -3.0;
    }
}

/// Configure the shared chain geometry and motion before any engine creates
/// bodies.
fn chain(body: &mut Body, index: u32) {
    // Each twelve-link group has an independent fixed root.
    let link = index % 12;
    body.position = Vec3::new((index / 12) as f32 * 2.0, 12.0 - link as f32 * 0.8, 0.0);
    body.shape = Shape::Box {
        half_extents: [0.3; 3],
    };
    // Adjacent anchors meet halfway across the eight-tenths-metre spacing.
    if link == 0 {
        body.kind = BodyKind::Static;
    } else {
        body.joint = Some(Joint {
            parent: index as usize - 1,
            parent_anchor: Vec3::NEG_Y * 0.4,
            child_anchor: Vec3::Y * 0.4,
        });
    }
}

/// Configure the shared character geometry and motion before any engine creates
/// bodies.
fn character(body: &mut Body, index: u32) {
    // One in four bodies is an inclined static obstacle for the controllers.
    if index.is_multiple_of(4) {
        body.kind = BodyKind::Static;
        body.position = Vec3::new(index as f32 * 2.0 + 4.0, 0.25, 0.0);
        body.shape = Shape::Box {
            half_extents: [1.5, 0.2, 2.0],
        };
        body.rotation = Quat::from_rotation_z(0.2);
    } else {
        // Controller spheres start above the floor with room for downward motion.
        body.kind = BodyKind::Kinematic;
        body.shape = Shape::Ball { radius: 0.35 };
        body.position = Vec3::new(index as f32 * 2.0, 1.0, 0.0);
    }
}

/// Configure the shared inverse kinematics geometry and motion before any
/// engine creates bodies.
fn inverse_kinematics(body: &mut Body, index: u32) {
    // Three markers identify the root, elbow, and endpoint of one limb.
    let link = index % 3;
    let origin = Vec3::new((index / 3) as f32 * 3.0, 2.0, 0.0);
    body.kind = if link == 0 {
        BodyKind::Static
    } else {
        BodyKind::Kinematic
    };
    body.position = origin + Vec3::X * link as f32;
    body.shape = Shape::Ball { radius: 0.1 };
    // Partial final limbs retain only the requested marker and overlay links.
    if link != 0 {
        body.joint = Some(Joint {
            parent: index as usize - 1,
            parent_anchor: Vec3::ZERO,
            child_anchor: Vec3::ZERO,
        });
    }
}
/// Create six anatomical parts with shared point anchors and no motors.
fn configure_ragdoll(body: &mut Body, index: u32, width: u32, dimension: Dimension) {
    // Six parts form one humanoid; spatial groups occupy a separated lattice.
    let part = index % 6;
    let group = index / 6;
    let origin = if dimension == Dimension::Two {
        Vec3::new(group as f32 * 2.4, 4.0, 0.0)
    } else {
        Vec3::new(
            (group % width) as f32 * 2.4,
            4.0 + (group / (width * width)) as f32 * 3.0,
            ((group / width) % width) as f32 * 2.4,
        )
    };
    // Offsets and anchors are in metres relative to the torso centre.
    let (offset, anchor, half_extents) = ragdoll_part(part);
    body.position = origin + offset;
    body.shape = if part == 1 {
        Shape::Ball { radius: 0.25 }
    } else {
        Shape::Box { half_extents }
    };
    // All limbs attach directly to the torso using their common world anchor.
    if part != 0 {
        body.joint = Some(Joint {
            parent: (index - part) as usize,
            parent_anchor: anchor,
            child_anchor: anchor - offset,
        });
    }
}

/// Return torso-relative offsets, anchors, and primitive half extents, all
/// measured in metres.
fn ragdoll_part(part: u32) -> (Vec3, Vec3, [f32; 3]) {
    match part {
        0 => (Vec3::ZERO, Vec3::ZERO, [0.3, 0.6, 0.2]),
        1 => (
            Vec3::new(0.0, 0.95, 0.0),
            Vec3::new(0.0, 0.65, 0.0),
            [0.25; 3],
        ),
        2 => (
            Vec3::new(-0.65, 0.2, 0.0),
            Vec3::new(-0.35, 0.5, 0.0),
            [0.18, 0.5, 0.18],
        ),
        3 => (
            Vec3::new(0.65, 0.2, 0.0),
            Vec3::new(0.35, 0.5, 0.0),
            [0.18, 0.5, 0.18],
        ),
        4 => (
            Vec3::new(-0.2, -1.15, 0.0),
            Vec3::new(-0.2, -0.6, 0.0),
            [0.18, 0.5, 0.18],
        ),
        _ => (
            Vec3::new(0.2, -1.15, 0.0),
            Vec3::new(0.2, -0.6, 0.0),
            [0.18, 0.5, 0.18],
        ),
    }
}

/// Leave one quarter of the shared lattice dynamic among fixed colliders.
fn static_field(body: &mut Body, index: u32) {
    if !index.is_multiple_of(4) {
        body.kind = BodyKind::Static;
    }
}

/// Aim a small ball downward at the shared thin floor.
fn projectile(body: &mut Body) {
    body.position.y += 3.0;
    body.shape = Shape::Ball { radius: 0.12 };
    body.velocity.y = -120.0;
}

/// Change only motion ownership for primitive, static-field, and query workloads.
fn primitive_motion(body: &mut Body, index: u32, scenario: Scenario) {
    if scenario == Scenario::StaticField {
        static_field(body, index);
    }
    if scenario == Scenario::Queries {
        body.kind = BodyKind::Static;
    }
}

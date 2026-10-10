//! Analytic two-bone inverse kinematics executed as Bevy animation work.

use crate::{Dimension, Settings, scene::Body};
use bevy::prelude::*;

/// Store only the joint markers that exist in a possibly partial final limb.
enum Markers {
    /// A final root with no requested child marker.
    Root,
    /// A root and elbow when the requested count leaves two markers.
    Elbow(Entity),
    /// A complete root, elbow, and end effector.
    Complete {
        /// Entity displaying the solved midpoint between the two segments.
        elbow: Entity,
        /// Entity displaying the moving target after enforcing the limb reach.
        end: Entity,
    },
}

/// Keep each limb's fixed anchor and the entities affected by its solution.
struct Rig {
    /// Root position in metres; both segment lengths are one metre.
    anchor: Vec3,
    /// Child markers present in the exact requested body count.
    markers: Markers,
}

/// Own the limb list and whether targets may leave the XY plane.
#[derive(Resource)]
struct Rigs {
    /// Limbs in stable scene order.
    limbs: Vec<Rig>,
    /// Coordinate space selected for the workload.
    dimension: Dimension,
}

/// Create renderable markers without loading a physics backend or collision
/// world.
pub(super) fn build(app: &mut App, settings: &Settings, bodies: &[Body]) -> Vec<Entity> {
    // Markers preserve the shared body order without adding physics components.
    let entities: Vec<_> = bodies
        .iter()
        .map(|body| {
            app.world_mut()
                .spawn(Transform::from_translation(body.position))
                .id()
        })
        .collect();
    let limbs = entities
        .chunks(3)
        .zip(bodies.iter().step_by(3))
        .map(|(entities, body)| {
            let markers = match entities {
                [_, elbow, end] => Markers::Complete {
                    elbow: *elbow,
                    end: *end,
                },
                [_, elbow] => Markers::Elbow(*elbow),
                _ => Markers::Root,
            };
            Rig {
                anchor: body.position,
                markers,
            }
        })
        .collect();
    app.insert_resource(Rigs {
        limbs,
        dimension: settings.input().dimension,
    });
    app.add_systems(PostUpdate, solve);
    entities
}

/// Solve once per limb and synchronize its child markers during the
/// measured step.
fn solve(rigs: Res<'_, Rigs>, time: Res<'_, Time>, mut transforms: Query<'_, '_, &mut Transform>) {
    // Offset each limb phase while retaining a fixed simulation-time trajectory.
    for (index, rig) in rigs.limbs.iter().enumerate() {
        let phase = time.elapsed_secs() + index as f32 * 0.17;
        let z = if rigs.dimension == Dimension::Three {
            phase.sin() * 0.4
        } else {
            0.0
        };
        // Planar targets keep zero depth; spatial targets move through a narrow depth range.
        let target = rig.anchor + Vec3::new(phase.sin() * 1.2, phase.cos() * 0.5, z);
        let (elbow_position, end_position) = two_bone(rig.anchor, target);
        // A partial final limb updates only markers that were actually requested.
        match rig.markers {
            Markers::Root => {}
            Markers::Elbow(elbow) => {
                if let Ok(mut transform) = transforms.get_mut(elbow) {
                    transform.translation = elbow_position;
                }
            }
            Markers::Complete { elbow, end } => {
                if let Ok(mut transform) = transforms.get_mut(elbow) {
                    transform.translation = elbow_position;
                }
                if let Ok(mut transform) = transforms.get_mut(end) {
                    transform.translation = end_position;
                }
            }
        }
    }
}

/// Intersect two unit-radius spheres, choosing a stable elbow plane and
/// clamping reach.
fn two_bone(anchor: Vec3, target: Vec3) -> (Vec3, Vec3) {
    // Clamp reach in metres to avoid coincident or fully extended singularities.
    let delta = target - anchor;
    let distance = delta.length().clamp(0.0001, 1.9999);
    let direction = delta.try_normalize().unwrap_or(Vec3::X);
    let pole = if direction.dot(Vec3::Y).abs() < 0.99 {
        Vec3::Y
    } else {
        Vec3::X
    };
    // Project a nonparallel pole to select one reproducible elbow plane.
    let perpendicular = (pole - direction * direction.dot(pole)).normalize();
    let midpoint = anchor + direction * (distance * 0.5);
    let height = (1.0 - distance * distance * 0.25).sqrt();
    (
        midpoint + perpendicular * height,
        anchor + direction * distance,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singular_and_unreachable_targets_keep_both_segment_lengths() {
        for target in [Vec3::ZERO, Vec3::X * 10.0, Vec3::Y, Vec3::NEG_Y] {
            let (elbow, end) = two_bone(Vec3::ZERO, target);
            assert!((elbow.length() - 1.0).abs() < 0.00001);
            assert!(((end - elbow).length() - 1.0).abs() < 0.00001);
        }
    }
}

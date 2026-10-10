//! Rapier 2D integration including Bevy synchronization.

use crate::{
    BodyKind, CollisionMode, Scenario, Settings, Shape, Sleeping as SleepingPolicy, scene::Body,
};
use bevy::prelude::*;
use bevy_rapier2d::{prelude::*, rapier::dynamics::IntegrationParameters};

/// Construct the engine with the same geometry and material as Avian.
pub(super) fn build(
    app: &mut App,
    settings: &Settings,
    bodies: &[Body],
    floor: Vec3,
) -> Result<Vec<Entity>, crate::BenchError> {
    // Set engine-specific solver effort without claiming equivalent solver work.
    let input = settings.input();
    let initialization = RapierContextInitialization::InitializeDefaultRapierContext {
        integration_parameters: IntegrationParameters {
            num_solver_iterations: input.solver_effort as usize,
            ..default()
        },
        rapier_configuration: RapierConfiguration {
            gravity: Vec2::NEG_Y * input.gravity,
            ..RapierConfiguration::new(1.0)
        },
    };
    app.add_plugins(
        RapierPhysicsPlugin::<NoUserData>::default().with_custom_initialization(initialization),
    );
    app.insert_resource(TimestepMode::Fixed {
        dt: settings.timestep().as_secs_f32(),
        substeps: 1,
    });
    spawn_floor(app.world_mut(), settings, floor);
    // Stable creation order lets joint descriptions refer to prior bodies.
    let entities: Vec<Entity> = bodies
        .iter()
        .map(|body| spawn_body(app.world_mut(), &input, body))
        .collect();
    // Rapier point joints use the same anchors as the Avian constraints.
    attach_joints(app, bodies, &entities)?;
    // Add only the scheduled work required by this scene.
    if input.scenario == Scenario::Kinematics {
        app.add_systems(PostUpdate, drive.before(PhysicsSet::SyncBackend));
    }
    if input.scenario == Scenario::Characters {
        app.add_systems(PostUpdate, characters.before(PhysicsSet::SyncBackend));
    }
    if input.scenario == Scenario::Queries {
        app.add_systems(PostUpdate, query_rays.after(PhysicsSet::Writeback));
    }
    Ok(entities)
}

/// Attach passive constraints after every referenced entity has been
/// created in stable order.
fn attach_joints(
    app: &mut App,
    bodies: &[Body],
    entities: &[Entity],
) -> Result<(), crate::BenchError> {
    // Resolve all parent indices against the completed entity list.
    bodies.iter().zip(entities).try_for_each(|(body, &child)| {
        if let Some(joint) = body.joint {
            // Connected partners share anchors and suppress their mutual contacts.
            let parent = super::joint_parent(entities, joint.parent)?;
            let mut constraint = RevoluteJointBuilder::new()
                .local_anchor1(joint.parent_anchor.truncate())
                .local_anchor2(joint.child_anchor.truncate())
                .build();
            constraint.data.set_contacts_enabled(false);
            app.world_mut()
                .get_entity_mut(child)
                .map_err(super::integration_error)?
                .insert(ImpulseJoint::new(parent, constraint));
        }
        Ok(())
    })
}

/// Convert one primitive and its motion policy into engine components
/// before simulation starts.
pub(super) fn spawn_body(world: &mut World, input: &crate::SettingsInput, body: &Body) -> Entity {
    // Preserve the shared primitive dimensions at the collider boundary.
    let collider = match body.shape {
        Shape::Box { half_extents } => Collider::cuboid(half_extents[0], half_extents[1]),
        Shape::Ball { radius } => Collider::ball(radius),
    };
    // Motion policy selects the backend body category before spawning.
    let rigid_body = match body.kind {
        BodyKind::Static => RigidBody::Fixed,
        BodyKind::Dynamic => RigidBody::Dynamic,
        BodyKind::Kinematic => RigidBody::KinematicVelocityBased,
    };
    let mut entity = world.spawn((
        rigid_body,
        collider,
        Transform::from_translation(body.position).with_rotation(body.rotation),
        Friction::coefficient(input.friction),
        Restitution::coefficient(input.restitution),
        Velocity {
            linear: body.velocity.truncate(),
            angular: body.angular_velocity.z,
        },
    ));
    // Optional components enable the requested deactivation and collision policies.
    entity.insert(if input.sleeping == SleepingPolicy::Disabled {
        Sleeping::disabled()
    } else {
        Sleeping::default()
    });
    if input.ccd == CollisionMode::Continuous {
        entity.insert(Ccd::enabled());
    }
    if body.kind == BodyKind::Kinematic {
        entity.insert(super::Driven);
    }

    if input.scenario == Scenario::Characters && body.kind == BodyKind::Kinematic {
        entity.insert(super::Character);
        entity.insert((
            RigidBody::KinematicPositionBased,
            KinematicCharacterController::default(),
        ));
    }
    entity.id()
}

/// Prescribe the same platform motion used by the Avian adapter.
fn drive(time: Res<'_, Time>, mut bodies: Query<'_, '_, &mut Velocity, With<super::Driven>>) {
    let speed = time.elapsed_secs().cos() * 2.0;
    for mut velocity in &mut bodies {
        velocity.linear.x = speed;
    }
}

/// Supply requested displacement before Rapier performs its controller casts.
fn characters(
    time: Res<'_, Time>,
    mut bodies: Query<'_, '_, &mut KinematicCharacterController, With<super::Character>>,
) {
    let speed = time.elapsed_secs().cos() * 2.0;
    for mut controller in &mut bodies {
        controller.translation = Some(Vec2::new(speed, -4.0) * time.delta_secs());
    }
}

/// Cast the shared rays against the completed backend query pipeline.
fn query_rays(
    context: ReadRapierContext<'_, '_>,
    origins: Res<'_, super::QueryOrigins>,
    mut statistics: ResMut<'_, crate::QueryStatistics>,
) {
    statistics.rays = origins.0.len();
    statistics.hits = 0;
    if let Ok(context) = context.single() {
        context.with_query_pipeline(QueryFilter::default(), |pipeline| {
            statistics.hits = origins
                .0
                .iter()
                .filter(|origin| {
                    pipeline
                        .cast_ray(origin.truncate(), Vec2::NEG_Y, 1.5, true)
                        .is_some()
                })
                .count();
        });
    }
}

/// Construct the same finite floor used by the other engine adapters.
fn spawn_floor(world: &mut World, settings: &Settings, floor: Vec3) {
    // Use the shared floor selected before backend construction.
    let input = settings.input();
    let extent = floor.x;
    let thickness = floor.y * 2.0;
    // Place the upper surface at zero metres and preserve material coefficients.
    world.spawn((
        RigidBody::Fixed,
        Collider::cuboid(extent, thickness * 0.5),
        Transform::from_xyz(0.0, -thickness * 0.5, 0.0),
        Friction::coefficient(input.friction),
        Restitution::coefficient(input.restitution),
    ));
}

/// Add world velocity and wake an existing dynamic entity.
pub(super) fn kick(
    world: &mut World,
    entity: Entity,
    delta: Vec3,
) -> Result<(), crate::BenchError> {
    // Resolve the owned entity before modifying either component.
    let mut body = world
        .get_entity_mut(entity)
        .map_err(super::integration_error)?;
    body.get_mut::<Velocity>()
        .expect("dynamic velocity exists")
        .linear += delta.truncate();
    body.get_mut::<Sleeping>()
        .expect("sleep policy exists")
        .sleeping = false;
    Ok(())
}

//! Avian 2D integration with explicit shared settings.

use crate::{
    BodyKind, CollisionMode, Scenario, Settings, Shape, Sleeping as SleepingPolicy, scene::Body,
};
use avian2d::prelude::*;
use bevy::prelude::*;

/// Construct the engine and seed its colliders outside timed samples.
pub(super) fn build(
    app: &mut App,
    settings: &Settings,
    bodies: &[Body],
) -> Result<Vec<Entity>, crate::BenchError> {
    // Configure the schedule before constructing any body or joint.
    let input = settings.input();
    app.add_plugins(PhysicsPlugins::new(PostUpdate));
    app.insert_resource(SubstepCount(input.solver_effort));
    app.insert_resource(Gravity(Vec2::NEG_Y * input.gravity));
    spawn_floor(app.world_mut(), settings, bodies);
    // Body order is shared with the renderer and every joint parent index.
    let entities: Vec<Entity> = bodies
        .iter()
        .map(|body| spawn_body(app.world_mut(), &input, body))
        .collect();
    // Disable contact response only between directly constrained partners.
    attach_joints(app, bodies, &entities)?;
    // Add only the scheduled work required by this scene.
    if input.scenario == Scenario::Kinematics {
        app.add_systems(PostUpdate, drive.before(PhysicsSystems::First));
    }
    if input.scenario == Scenario::Characters {
        app.add_systems(PostUpdate, characters.before(PhysicsSystems::First));
    }
    if input.scenario == Scenario::Queries {
        app.add_systems(PostUpdate, query_rays.after(PhysicsSystems::Last));
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
            app.world_mut().spawn((
                RevoluteJoint::new(parent, child)
                    .with_local_anchor1(joint.parent_anchor.truncate())
                    .with_local_anchor2(joint.child_anchor.truncate()),
                JointCollisionDisabled,
            ));
        }
        Ok(())
    })
}

/// Convert one primitive and its motion policy into engine components
/// before simulation starts.
fn spawn_body(world: &mut World, input: &crate::SettingsInput, body: &Body) -> Entity {
    // Preserve the shared primitive dimensions at the collider boundary.
    let collider = match body.shape {
        Shape::Box { half_extents } => {
            Collider::rectangle(half_extents[0] * 2.0, half_extents[1] * 2.0)
        }
        Shape::Ball { radius } => Collider::circle(radius),
    };
    // Motion policy selects the backend body category before spawning.
    let rigid_body = match body.kind {
        BodyKind::Static => RigidBody::Static,
        BodyKind::Dynamic => RigidBody::Dynamic,
        BodyKind::Kinematic => RigidBody::Kinematic,
    };
    let mut entity = world.spawn((
        rigid_body,
        collider,
        Transform::from_translation(body.position).with_rotation(body.rotation),
        Friction::new(input.friction),
        Restitution::new(input.restitution),
        LinearVelocity(body.velocity.truncate()),
        AngularVelocity(body.angular_velocity.z),
    ));
    // Optional components enable the requested deactivation and collision policies.
    if input.sleeping == SleepingPolicy::Disabled {
        entity.insert(SleepingDisabled);
    }
    if input.ccd == CollisionMode::Continuous {
        entity.insert(SweptCcd::default());
    }
    if body.kind == BodyKind::Kinematic {
        entity.insert(super::Driven);
    }

    if input.scenario == Scenario::Characters && body.kind == BodyKind::Kinematic {
        entity.insert(super::Character);
        entity.insert(CustomPositionIntegration);
    }
    entity.id()
}

/// Set platform velocity from simulation time, independent of rendering speed.
fn drive(time: Res<'_, Time>, mut bodies: Query<'_, '_, &mut LinearVelocity, With<super::Driven>>) {
    let speed = time.elapsed_secs().cos() * 2.0;
    for mut velocity in &mut bodies {
        velocity.0.x = speed;
    }
}

/// Move each controller with the backend's shape-cast and slide algorithm.
fn characters(
    time: Res<'_, Time>,
    move_and_slide: MoveAndSlide<'_, '_>,
    mut bodies: Query<'_, '_, (Entity, &Collider, &mut Transform), With<super::Character>>,
) {
    // Use simulation time so controller requests do not depend on render cadence.
    let speed = time.elapsed_secs().cos() * 2.0;
    for (entity, collider, mut transform) in &mut bodies {
        let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
        // Exclude the controller itself before resolving its desired displacement.
        let output = move_and_slide.move_and_slide(
            collider,
            transform.translation.truncate(),
            transform.rotation.to_euler(EulerRot::XYZ).2,
            Vec2::new(speed, -4.0),
            time.delta(),
            &MoveAndSlideConfig::default(),
            &filter,
            |_| MoveAndSlideHitResponse::Accept,
        );
        transform.translation = output.position.extend(0.0);
    }
}

/// Cast the same downward rays after the engine synchronizes its query data.
fn query_rays(
    spatial: SpatialQuery<'_, '_>,
    origins: Res<'_, super::QueryOrigins>,
    mut statistics: ResMut<'_, crate::QueryStatistics>,
) {
    statistics.rays = origins.0.len();
    statistics.hits = origins
        .0
        .iter()
        .filter(|origin| {
            spatial
                .cast_ray(
                    origin.truncate(),
                    Dir2::NEG_Y,
                    1.5,
                    true,
                    &SpatialQueryFilter::default(),
                )
                .is_some()
        })
        .count();
}

/// Construct the same finite floor used by the other engine adapters.
fn spawn_floor(world: &mut World, settings: &Settings, bodies: &[Body]) {
    // Compute a finite floor from the shared initial geometry.
    let input = settings.input();
    let floor = crate::scene::floor_half_extents(bodies, settings);
    let extent = floor.x;
    let thickness = floor.y * 2.0;
    // Place the upper surface at zero metres and preserve material coefficients.
    world.spawn((
        RigidBody::Static,
        Collider::rectangle(extent * 2.0, thickness),
        Transform::from_xyz(0.0, -thickness * 0.5, 0.0),
        Friction::new(input.friction),
        Restitution::new(input.restitution),
    ));
}

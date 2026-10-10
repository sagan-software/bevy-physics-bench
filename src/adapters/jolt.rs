//! Native Jolt integration with a Bevy transform synchronization boundary.

use super::integration_error;
use crate::{
    BenchError, BodyKind, CollisionMode, Scenario, Settings, Shape, Sleeping, scene::Body,
};
use bevy::prelude::*;
use oxijolt::{self as jolt, BodySettings, PhysicsWorld, WorldSettings};
use std::sync::Arc;

/// Own the native world and map its handles to Bevy render entities.
struct JoltWorld {
    /// Jolt owns its bodies, shapes, constraints, and character controllers.
    world: PhysicsWorld,
    /// Handles follow the common scene's stable creation order.
    handles: BodyHandles,
    /// Validated configuration for the controller and query workloads.
    settings: Settings,
}

/// Native handles paired with the Bevy entity and immutable initial geometry.
type BodyHandles = Vec<(Entity, Handle, Body)>;

/// Distinguish controller ownership from ordinary rigid-body ownership.
#[derive(Clone, Copy)]
enum Handle {
    /// A body stored in Jolt's rigid-body set.
    Body(jolt::BodyId),
    /// A virtual character stored in Jolt's controller registry.
    Character(jolt::CharacterId),
}

/// Execute Jolt jobs on the calling thread for the shared baseline profile.
struct InlineJobs;

/// Append unjointed primitives while preserving the owned native world.
pub(super) fn append(
    app: &mut App,
    input: &crate::SettingsInput,
    bodies: &[Body],
) -> Result<Vec<Entity>, BenchError> {
    let mut state = app
        .world_mut()
        .remove_non_send::<JoltWorld>()
        .expect("Jolt world exists");
    let result = create_bodies(app, &mut state.world, bodies, input).map(|(handles, entities)| {
        state.handles.extend(handles);
        entities
    });
    // Restore ownership even when the backend rejects an allocation.
    app.insert_non_send(state);
    result
}

/// Apply a velocity kick through the native handle, which wakes the body.
pub(super) fn kick(world: &mut World, kick: crate::BodyKick) -> Result<(), BenchError> {
    // Resolve the native world and stable handle before changing velocity.
    let mut state = world
        .get_non_send_mut::<JoltWorld>()
        .expect("Jolt world exists");
    let Handle::Body(id) = state
        .handles
        .get(kick.index())
        .expect("validated Jolt body index")
        .1
    else {
        return Err(BenchError::Unsupported("kick target must be a rigid body"));
    };
    // Preserve existing velocity; the native setter wakes a moving body.
    let mut body = state.world.body_mut(id).map_err(integration_error)?;
    let old = body.linear_velocity();
    let delta = kick.delta();
    body.set_linear_velocity(jolt::Vec3::new(
        old.x + delta.x,
        old.y + delta.y,
        old.z + delta.z,
    ))
    .map_err(integration_error)
}

impl jolt::JobSystem for InlineJobs {
    fn max_concurrency(&self) -> u32 {
        1
    }
    fn queue_job(&self, job: jolt::Job) {
        job.run();
    }
}

/// Construct bodies and constraints before any work enters a timing interval.
pub(super) fn build(
    app: &mut App,
    settings: &Settings,
    bodies: &[Body],
    capacity: crate::BodyCount,
) -> Result<Vec<Entity>, BenchError> {
    let input = settings.input();
    // Published Oxijolt exposes full substeps; Jolt retains ten velocity and two position passes.
    let configuration = WorldSettings::default()
        .max_bodies(capacity.get() + 1)
        .job_system(Arc::new(InlineJobs))
        .gravity(jolt::Vec3::new(0.0, -input.gravity, 0.0));
    // Allocate the native world and its floor before adding scenario bodies.
    let mut world = PhysicsWorld::new(configuration).map_err(integration_error)?;
    create_floor(&mut world, settings, bodies)?;
    let (handles, entities) = create_bodies(app, &mut world, bodies, &input)?;
    // Shared anchors use local centre-of-mass coordinates for primitive shapes.
    attach_joints(&mut world, bodies, &handles)?;
    // Optimize static geometry once, outside every measured step.
    world.optimize_broad_phase();
    app.insert_non_send(JoltWorld {
        world,
        handles,
        settings: *settings,
    });
    app.add_systems(PostUpdate, step);
    Ok(entities)
}

/// Disable contact only for directly connected parts within each
/// independent group.
fn joint_filter(
    bodies: &[Body],
    group_size: u32,
) -> Result<Option<jolt::GroupFilterTable>, BenchError> {
    // Scenes without joints need no collision-group allocation.
    if !bodies.iter().any(|body| body.joint.is_some()) {
        return Ok(None);
    }
    let mut builder = jolt::GroupFilterTableBuilder::new(group_size).map_err(integration_error)?;
    // Only direct constraint partners suppress contacts within their group.
    for (index, body) in bodies.iter().enumerate() {
        if let Some(joint) = body.joint {
            builder
                .disable_collision(joint.parent as u32 % group_size, index as u32 % group_size)
                .map_err(integration_error)?;
        }
    }
    Ok(Some(builder.build()))
}

/// Retain errors for the public step call instead of allowing Bevy to
/// discard them.
fn step(
    mut state: NonSendMut<'_, JoltWorld>,
    time: Res<'_, Time>,
    mut transforms: Query<'_, '_, (&mut Transform, &mut super::JoltSleeping)>,
    mut statistics: ResMut<'_, crate::QueryStatistics>,
    mut status: ResMut<'_, super::EngineStatus>,
) {
    if let Err(error) = update(&mut state, &time, transforms.reborrow(), &mut statistics) {
        status.0 = Some(error);
    }
}

/// Advance the native world and synchronize its completed poses into Bevy.
fn update(
    state: &mut JoltWorld,
    time: &Time,
    mut transforms: Query<'_, '_, (&mut Transform, &mut super::JoltSleeping)>,
    statistics: &mut crate::QueryStatistics,
) -> Result<(), BenchError> {
    // Drivers consume the same fixed time as the rigid-body update.
    let input = state.settings.input();
    drive_handles(state, time)?;
    advance_world(&mut state.world, time.delta_secs(), input.solver_effort)?;
    // Publish completed poses before collecting workload counters.
    synchronize(state, transforms.reborrow())?;
    if input.scenario == Scenario::Queries {
        query_rays(state, statistics)?;
    }
    Ok(())
}

/// Convert a Bevy vector at the native binding boundary without retaining
/// primitives.
fn vector(value: Vec3) -> jolt::Vec3 {
    jolt::Vec3::new(value.x, value.y, value.z)
}

/// Convert a world or local point into Jolt's single-precision position
/// representation.
fn point(value: Vec3) -> jolt::RVec3 {
    jolt::RVec3::new(value.x, value.y, value.z)
}

/// Construct one Jolt body or controller from the shared primitive description.
fn create_handle(
    world: &mut PhysicsWorld,
    body: Body,
    index: usize,
    input: &crate::SettingsInput,
    filter: Option<&jolt::GroupFilterTable>,
    group_size: u32,
) -> Result<Handle, BenchError> {
    // Jolt uses zero convex radius so box geometry matches the shared primitive.
    let shape = match body.shape {
        Shape::Box { half_extents } => jolt::Shape::new_box_with_convex_radius(
            jolt::Vec3::new(half_extents[0], half_extents[1], half_extents[2]),
            0.0,
        ),
        Shape::Ball { radius } => jolt::Shape::new_sphere(radius),
    }
    .map_err(integration_error)?;
    // Convert pose coordinates at the native binding boundary.
    let position = point(body.position);
    let rotation = rotation(body.rotation);
    // Character scenes use virtual controllers rather than rigid kinematic motion.
    let handle = if input.scenario == Scenario::Characters && body.kind == BodyKind::Kinematic {
        let configuration = jolt::CharacterSettings::new(&shape).collide_with_characters(true);
        Handle::Character(
            world
                .create_character(&configuration, position, rotation)
                .map_err(integration_error)?,
        )
    } else {
        let mut configuration = body_settings(body, input);
        if let Some(filter) = filter {
            configuration = configuration.collision_group(
                jolt::CollisionGroup::new(
                    filter,
                    index as u32 / group_size,
                    index as u32 % group_size,
                )
                .map_err(integration_error)?,
            );
        }
        Handle::Body(
            world
                .create_body(&shape, &configuration)
                .map_err(integration_error)?,
        )
    };
    Ok(handle)
}

/// Apply common material, velocity, and mass units before inserting a rigid
/// body.
fn body_settings(body: Body, input: &crate::SettingsInput) -> BodySettings {
    // Motion categories are closed and preserve static versus driven ownership.
    let configuration = match body.kind {
        BodyKind::Static => BodySettings::new_static(),
        BodyKind::Dynamic => BodySettings::new_dynamic(),
        BodyKind::Kinematic => BodySettings::new_kinematic(),
    };
    let mut configuration = configuration
        .position(point(body.position))
        .rotation(rotation(body.rotation))
        .linear_velocity(vector(body.velocity))
        .angular_velocity(vector(body.angular_velocity))
        .linear_damping(0.0)
        .angular_damping(0.0)
        .friction(input.friction)
        .restitution(input.restitution)
        .allow_sleeping(input.sleeping == Sleeping::Enabled)
        .motion_quality(if input.ccd == CollisionMode::Continuous {
            jolt::MotionQuality::LinearCast
        } else {
            jolt::MotionQuality::Discrete
        });
    // Density is one mass unit per cubic metre, so mass equals primitive volume.
    if body.kind != BodyKind::Static {
        let volume = match body.shape {
            Shape::Box { half_extents } => {
                8.0 * half_extents[0] * half_extents[1] * half_extents[2]
            }
            Shape::Ball { radius } => 4.0 / 3.0 * std::f32::consts::PI * radius.powi(3),
        };
        configuration = configuration.mass(volume);
    }
    configuration
}

/// Convert the shared unit quaternion using Jolt's explicit XYZW constructor.
fn rotation(value: Quat) -> jolt::Quat {
    jolt::Quat::from_xyzw(value.x, value.y, value.z, value.w)
}

/// Place the common floor with its upper face at zero metres on the vertical
/// axis.
fn create_floor(
    world: &mut PhysicsWorld,
    settings: &Settings,
    bodies: &[Body],
) -> Result<(), BenchError> {
    let input = settings.input();
    // The shared extent includes scenario-specific margins, including CCD projectiles.
    let floor_extents = crate::scene::floor_half_extents(bodies, settings);
    let extent = floor_extents.x;
    let thickness = floor_extents.y * 2.0;
    let floor = jolt::Shape::new_box_with_convex_radius(
        jolt::Vec3::new(extent, thickness * 0.5, extent),
        0.0,
    )
    .map_err(integration_error)?;
    // Half-thickness offsets the centre so the upper surface remains at zero.
    world
        .create_body(
            &floor,
            &BodySettings::new_static()
                .position(jolt::RVec3::new(0.0, -thickness * 0.5, 0.0))
                .friction(input.friction)
                .restitution(input.restitution),
        )
        .map_err(integration_error)?;
    Ok(())
}

/// Register common point anchors after all rigid bodies exist in the native
/// world.
fn attach_joints(
    world: &mut PhysicsWorld,
    bodies: &[Body],
    handles: &[(Entity, Handle, Body)],
) -> Result<(), BenchError> {
    // Resolve joint parents only after all handles have been constructed.
    for (body, &(_, child, _)) in bodies.iter().zip(handles) {
        if let Some(joint) = body.joint {
            let parent = handles
                .get(joint.parent)
                .map(|entry| entry.1)
                .ok_or_else(|| {
                    integration_error(std::io::Error::other("generated joint parent is absent"))
                })?;
            let (Handle::Body(parent), Handle::Body(child)) = (parent, child) else {
                return Err(BenchError::Unsupported(
                    "point constraints require rigid bodies",
                ));
            };
            // Primitive centres coincide with local centres of mass.
            let constraint = jolt::PointConstraintSettings::default()
                .space(jolt::ConstraintSpace::LocalToBodyCom)
                .point1(point(joint.parent_anchor))
                .point2(point(joint.child_anchor));
            world
                .create_constraint(parent, child, &constraint)
                .map_err(integration_error)?;
        }
    }
    Ok(())
}

/// Request identical controller and moving-platform velocities before advancing
/// the native world.
fn drive_handles(state: &mut JoltWorld, time: &Time) -> Result<(), BenchError> {
    // Every driver uses fixed simulation time, independent of rendering cadence.
    let input = state.settings.input();
    let speed = time.elapsed_secs().cos() * 2.0;
    // Controllers request the same metres-per-second motion as the other adapters.
    for &(_, handle, body) in &state.handles {
        match handle {
            Handle::Character(id) => drive_character(
                &mut state.world,
                id,
                speed,
                time.delta_secs(),
                input.gravity,
            )?,
            Handle::Body(id) if body.kind == BodyKind::Kinematic => {
                state
                    .world
                    .body_mut(id)
                    .map_err(integration_error)?
                    .set_linear_velocity(jolt::Vec3::new(speed, 0.0, 0.0))
                    .map_err(integration_error)?;
            }
            Handle::Body(_) => {}
        }
    }
    Ok(())
}

/// Copy completed native poses into Bevy transforms inside the measured
/// integration step.
fn synchronize(
    state: &JoltWorld,
    mut transforms: Query<'_, '_, (&mut Transform, &mut super::JoltSleeping)>,
) -> Result<(), BenchError> {
    // Snapshot synchronization is included in this Bevy integration series.
    for &(entity, handle, _) in &state.handles {
        // Read the matching native object before borrowing its Bevy transform.
        let (position, rotation, is_sleeping) = match handle {
            Handle::Body(id) => {
                let body = state.world.body(id).map_err(integration_error)?;
                (body.position(), body.rotation(), !body.is_active())
            }
            Handle::Character(id) => {
                let character = state.world.character(id).map_err(integration_error)?;
                (character.position(), character.rotation(), false)
            }
        };
        let (mut transform, mut sleeping) =
            transforms.get_mut(entity).map_err(integration_error)?;
        // Copy XYZ and XYZW without changing coordinate conventions.
        transform.translation = Vec3::new(position.x, position.y, position.z);
        transform.rotation = Quat::from_xyzw(rotation.x, rotation.y, rotation.z, rotation.w);
        sleeping.0 = is_sleeping;
    }
    Ok(())
}

/// Seed native bodies and paired Bevy entities in their common scene order.
fn create_bodies(
    app: &mut App,
    world: &mut PhysicsWorld,
    bodies: &[Body],
    input: &crate::SettingsInput,
) -> Result<(BodyHandles, Vec<Entity>), BenchError> {
    // Collision groups separate independent chains and six-part ragdolls.
    let group_size = if input.scenario == Scenario::Ragdolls {
        6
    } else {
        12
    };
    let filter = joint_filter(bodies, group_size)?;
    // Allocate mappings once; scene order determines every joint parent.
    let mut handles = Vec::with_capacity(bodies.len());
    let mut entities = Vec::with_capacity(bodies.len());
    // Bevy entities carry render state; the native backend owns all physical state.
    for (index, body) in bodies.iter().copied().enumerate() {
        let handle = create_handle(world, body, index, input, filter.as_ref(), group_size)?;
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(body.position).with_rotation(body.rotation),
                super::JoltSleeping(false),
            ))
            .id();
        handles.push((entity, handle, body));
        entities.push(entity);
    }
    Ok((handles, entities))
}

/// Execute complete substeps and reject exhausted native contact buffers.
fn advance_world(world: &mut PhysicsWorld, seconds: f32, effort: u32) -> Result<(), BenchError> {
    // Divide seconds by substep count; each call retains Jolt's internal passes.
    for _ in 0..effort {
        let report = world
            .step(seconds / effort as f32)
            .map_err(integration_error)?;
        // Capacity exhaustion invalidates the step even if some jobs completed.
        if !report.is_complete() {
            return Err(integration_error(std::io::Error::other(
                "Jolt exhausted a contact buffer",
            )));
        }
    }
    Ok(())
}

/// Count shared downward ray hits after the native world has completed its
/// step.
fn query_rays(
    state: &JoltWorld,
    statistics: &mut crate::QueryStatistics,
) -> Result<(), BenchError> {
    // Reset counters because snapshots describe only the most recent update.
    statistics.rays = state.handles.len();
    statistics.hits = 0;
    // Alternate one body hit with one displaced miss in stable scene order.
    for (index, &(_, _, body)) in state.handles.iter().enumerate() {
        let ray = jolt::RayCast::new(
            jolt::RVec3::new(
                body.position.x + if index % 2 == 0 { 0.0 } else { 1_000.0 },
                body.position.y + 1.0,
                body.position.z,
            ),
            jolt::Vec3::new(0.0, -1.5, 0.0),
        );
        if state
            .world
            .cast_ray(&ray, &jolt::QueryFilter::default())
            .map_err(integration_error)?
            .is_some()
        {
            statistics.hits += 1;
        }
    }
    Ok(())
}

/// Advance one virtual controller after setting its requested velocity.
fn drive_character(
    world: &mut PhysicsWorld,
    id: jolt::CharacterId,
    speed: f32,
    seconds: f32,
    gravity: f32,
) -> Result<(), BenchError> {
    // Both velocity components are metres per second; vertical motion presses onto ramps.

    world
        .character_mut(id)
        .map_err(integration_error)?
        .set_linear_velocity(jolt::Vec3::new(speed, -4.0, 0.0))
        .map_err(integration_error)?;
    // Resolve collisions for the full fixed duration before rigid-body stepping.
    world
        .update_character(
            id,
            seconds,
            jolt::Vec3::new(0.0, -gravity, 0.0),
            &jolt::ExtendedUpdateSettings::default(),
            &jolt::QueryFilter::default(),
        )
        .map_err(integration_error)?;
    Ok(())
}

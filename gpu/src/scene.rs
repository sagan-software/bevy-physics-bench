//! Convert validated shared geometry into the published Nexus input profile.

use bevy_physics_bench::{
    BenchError, BodyKind, CollisionMode, Dimension, InitialBody, InitialScene, Scenario, Settings,
    SettingsInput, Shape, Sleeping,
};
use khal::backend::GpuBackend;
use nexus_rbd3d::{
    math::{Pose, Quat, Vec3},
    pipeline::{RbdCapacities, RbdResizePolicy, RbdState},
    rapier::prelude::*,
    shaders::dynamics::RbdSimParams,
};

/// Reject unsupported behavior before a device is requested, including
/// modes Nexus cannot reproduce.
pub(crate) fn validate(settings: &Settings) -> Result<(), BenchError> {
    // Reject profile mismatches before any device allocation or shader work.
    let input = settings.input();
    if input.dimension != Dimension::Three {
        return Err(BenchError::Unsupported("Nexus supports only 3D"));
    }
    if input.ccd != CollisionMode::Discrete {
        return Err(BenchError::Unsupported(
            "Nexus profile does not support continuous collision detection",
        ));
    }
    if input.sleeping != Sleeping::Disabled {
        return Err(BenchError::Unsupported(
            "Nexus profile does not support sleeping",
        ));
    }
    // Accepted workloads require only the implemented rigid-body and point-joint paths.
    match input.scenario {
        Scenario::Boxes
        | Scenario::Balls
        | Scenario::Pyramid
        | Scenario::Stacks
        | Scenario::StaticField
        | Scenario::Dominoes
        | Scenario::JointChain
        | Scenario::Ragdolls => Ok(()),
        Scenario::Kinematics
        | Scenario::Ccd
        | Scenario::Queries
        | Scenario::Characters
        | Scenario::InverseKinematics => Err(BenchError::Unsupported(
            "Nexus profile supports piles, stacks, static fields, dominoes, chains, and ragdolls",
        )),
    }
}

/// Convert the shared initial state once; the GPU owns all evolving
/// physical state afterward.
pub(crate) fn upload(backend: &GpuBackend, settings: &Settings, scene: &InitialScene) -> RbdState {
    let input = settings.input();
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let mut joints = ImpulseJointSet::new();
    let multibodies = MultibodyJointSet::new();
    // One collider belongs to each body, as required by Nexus 0.6.0's Rapier
    // conversion.
    // https://docs.rs/nexus_rbd3d/0.6.0/nexus_rbd3d/pipeline/struct.RbdState.html#method.from_rapier
    insert_floor(scene, &input, &mut bodies, &mut colliders);
    let mut handles = Vec::with_capacity(scene.bodies().len());
    for body in scene.bodies() {
        let handle = insert_body(body, &input, &mut bodies, &mut colliders);
        handles.push(handle);
    }
    insert_joints(scene, &handles, &mut joints);
    let mut parameters = RbdSimParams::tgs_soft();
    parameters.dt = settings.timestep().as_secs_f32();
    parameters.num_solver_iterations = input.solver_effort;
    // Bounded buffers are checked after every completed step; overflow
    // invalidates the run.
    let capacities = RbdCapacities {
        batches: 1,
        body_capacity: input.count.get() + 1,
        collisions_capacity: (input.count.get() + 1) * 32,
        mb_contact_constraints_capacity: 256,
        collisions_resize_policy: RbdResizePolicy::Fixed,
        solver_colors: 64,
        solver_colors_resize_policy: RbdResizePolicy::Fixed,
    };
    let mut state = RbdState::from_rapier(
        backend,
        &[(&bodies, &colliders, &joints, &multibodies, &parameters)],
        capacities,
    );
    state.set_gravity(backend, [0.0, -input.gravity, 0.0]);
    state
}

/// Insert the finite shared floor before scene bodies to reserve pose slot zero.
fn insert_floor(
    scene: &InitialScene,
    input: &SettingsInput,
    bodies: &mut RigidBodySet,
    colliders: &mut ColliderSet,
) {
    // Offset the centre by half-thickness so the upper surface remains at zero metres.
    let floor = scene.floor_half_extents();
    let ground =
        bodies.insert(RigidBodyBuilder::fixed().translation(Vec3::new(0.0, -floor.y, 0.0)));
    colliders.insert_with_parent(
        ColliderBuilder::cuboid(floor.x, floor.y, floor.z)
            .friction(input.friction)
            .restitution(input.restitution),
        ground,
        bodies,
    );
}

/// Convert one primitive into the same mass, material, and initial-motion profile.
fn insert_body(
    body: &InitialBody,
    input: &SettingsInput,
    bodies: &mut RigidBodySet,
    colliders: &mut ColliderSet,
) -> RigidBodyHandle {
    let handle = bodies.insert(body_builder(body));
    // Unit density and zero damping match the other shared-scene adapters.
    let shape = match body.shape() {
        Shape::Box {
            half_extents: [x, y, z],
        } => ColliderBuilder::cuboid(x, y, z),
        Shape::Ball { radius } => ColliderBuilder::ball(radius),
    };
    colliders.insert_with_parent(
        shape
            .density(1.0)
            .friction(input.friction)
            .restitution(input.restitution),
        handle,
        bodies,
    );
    handle
}

/// Insert passive point constraints after every scene handle exists in stable order.
fn insert_joints(scene: &InitialScene, handles: &[RigidBodyHandle], joints: &mut ImpulseJointSet) {
    // Primitive centres coincide with centres of mass; point constraints retain
    // local anchors.
    for (body, &child) in scene.bodies().iter().zip(handles) {
        if let Some(joint) = body.joint() {
            let constraint = SphericalJointBuilder::new()
                .local_anchor1(Vec3::from_array(joint.parent_anchor().to_array()))
                .local_anchor2(Vec3::from_array(joint.child_anchor().to_array()))
                .contacts_enabled(false);
            // InitialScene creates only earlier parent indices within the same group.
            let parent = *handles
                .get(joint.parent())
                .expect("generated joint parent exists");
            joints.insert(parent, child, constraint, true);
        }
    }
}

/// Convert motion ownership, pose, and velocities before inserting a backend body.
fn body_builder(body: &InitialBody) -> RigidBodyBuilder {
    // Motion ownership selects the Rapier construction type consumed by Nexus.
    let builder = match body.kind() {
        BodyKind::Static => RigidBodyBuilder::fixed(),
        BodyKind::Dynamic => RigidBodyBuilder::dynamic(),
        BodyKind::Kinematic => RigidBodyBuilder::kinematic_velocity_based(),
    };
    // Both math libraries use XYZ vectors and XYZW quaternions.
    let position = Vec3::from_array(body.position().to_array());
    let rotation = Quat::from_array(body.rotation().to_array());
    builder
        .pose(Pose::from_parts(position, rotation))
        .linvel(Vec3::from_array(body.linear_velocity().to_array()))
        .angvel(Vec3::from_array(body.angular_velocity().to_array()))
        .linear_damping(0.0)
        .angular_damping(0.0)
        .can_sleep(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_profile_accepts_each_rigid_workload_and_rejects_each_unsupported_mode() {
        for scenario in [
            "boxes",
            "balls",
            "pyramid",
            "stacks",
            "static-field",
            "dominoes",
            "joint-chain",
            "ragdolls",
        ] {
            let settings: Settings = format!(r#"{{"scenario":"{scenario}"}}"#).parse().unwrap();
            assert!(validate(&settings).is_ok());
        }
        for json in [
            r#"{"dimension":"2d"}"#,
            r#"{"sleeping":"enabled"}"#,
            r#"{"ccd":"continuous"}"#,
            r#"{"scenario":"kinematics"}"#,
            r#"{"scenario":"ccd"}"#,
            r#"{"scenario":"queries"}"#,
            r#"{"scenario":"characters"}"#,
            r#"{"scenario":"inverse-kinematics"}"#,
        ] {
            assert!(matches!(
                validate(&json.parse().unwrap()),
                Err(BenchError::Unsupported(_))
            ));
        }
    }
}

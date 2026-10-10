//! Interaction changes velocity without replacing the scene or advancing time.

use bevy_physics_bench::{BodyKick, Engine, Settings, Simulation, StepCount};

#[test]
fn growing_floor_retains_bodies_beyond_the_fixed_arena() {
    let settings: Settings = r#"{"scenario":"boxes","count":1}"#.parse().unwrap();
    let capacity = 2.try_into().unwrap();
    let fixed = bevy_physics_bench::InitialScene::new(&settings);
    let growing = bevy_physics_bench::InitialScene::with_capacity(&settings, capacity).unwrap();
    assert_eq!(fixed.floor_half_extents().x, 20.0);
    assert_eq!(
        growing.floor_half_extents().to_array(),
        [1000.0, 0.5, 1000.0]
    );
    assert_eq!(fixed.bodies()[0].position(), growing.bodies()[0].position());
    assert_ne!(
        bevy_physics_bench::SceneFingerprint::of(&settings).unwrap(),
        bevy_physics_bench::SceneFingerprint::with_capacity(&settings, capacity).unwrap()
    );
    for engine in [Engine::Avian, Engine::Rapier, Engine::Jolt] {
        if engine == Engine::Jolt && !cfg!(feature = "jolt") {
            continue;
        }
        let mut world = Simulation::with_capacity(engine, settings, capacity).unwrap();
        world
            .kick(BodyKick::try_from((0, [20.0, 0.0, 0.0])).unwrap())
            .unwrap();
        world.advance(StepCount::try_from(180).unwrap()).unwrap();
        let body = &world.snapshot().bodies[0];
        assert!(body.position[0] > 20.0, "{engine:?}: {body:?}");
        assert!(body.position[1] >= -0.25, "{engine:?}: {body:?}");
    }
}

#[test]
fn capacity_geometry_validates_bounds_and_preserves_fixed_fingerprints() {
    use bevy_physics_bench::{BenchError, InitialScene, SceneFingerprint};
    for scenario in ["boxes", "balls", "pyramid"] {
        let settings: Settings = format!(r#"{{"scenario":"{scenario}","count":2}}"#)
            .parse()
            .unwrap();
        let count = 2.try_into().unwrap();
        assert_eq!(
            SceneFingerprint::of(&settings).unwrap(),
            SceneFingerprint::with_capacity(&settings, count).unwrap()
        );
        assert!(matches!(
            InitialScene::with_capacity(&settings, 1.try_into().unwrap()),
            Err(BenchError::InvalidSetting(
                "capacity is smaller than the initial scene"
            ))
        ));
        assert!(matches!(
            SceneFingerprint::with_capacity(&settings, 1.try_into().unwrap()),
            Err(BenchError::InvalidSetting(
                "capacity is smaller than the initial scene"
            ))
        ));
        let grown = InitialScene::with_capacity(&settings, 3.try_into().unwrap()).unwrap();
        assert_eq!(
            grown.floor_half_extents().x,
            if scenario == "pyramid" { 20.0 } else { 1000.0 }
        );
    }
}

#[test]
fn kick_validation_rejects_malformed_values_and_disallowed_targets_before_mutation() {
    for delta in [
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::INFINITY, 0.0],
        [0.0, 0.0, -101.0],
        [100.01, 0.0, 0.0],
    ] {
        assert!(BodyKick::try_from((0, delta)).is_err());
    }
    assert!(BodyKick::try_from((32768, [0.0; 3])).is_err());
    for delta in [[-100.0; 3], [100.0; 3], [0.0; 3]] {
        let kick = BodyKick::try_from((32767, delta)).unwrap();
        assert_eq!(kick.index(), 32767);
        assert_eq!(kick.delta().to_array(), delta);
    }
    for (engine, json, index, delta) in [
        (Engine::Avian, r#"{"count":1}"#, 1, [1.0, 0.0, 0.0]),
        (
            Engine::Rapier,
            r#"{"count":4,"scenario":"static-field"}"#,
            1,
            [1.0, 0.0, 0.0],
        ),
        (
            Engine::Avian,
            r#"{"count":1,"dimension":"2d"}"#,
            0,
            [0.0, 0.0, 1.0],
        ),
        (
            Engine::Bevy,
            r#"{"count":3,"scenario":"inverse-kinematics"}"#,
            0,
            [1.0, 0.0, 0.0],
        ),
    ] {
        let mut world = Simulation::new(engine, json.parse().unwrap()).unwrap();
        let before = serde_json::to_value(world.snapshot()).unwrap();
        assert!(
            world
                .kick(BodyKick::try_from((index, delta)).unwrap())
                .is_err()
        );
        assert_eq!(serde_json::to_value(world.snapshot()).unwrap(), before);
    }
}

#[test]
fn growth_rejects_invalid_capacity_and_nonpile_workloads() {
    assert!(
        Simulation::with_capacity(Engine::Avian, Settings::default(), 1.try_into().unwrap())
            .is_err()
    );
    let settings: Settings = r#"{"scenario":"pyramid","count":1}"#.parse().unwrap();
    let mut world =
        Simulation::with_capacity(Engine::Avian, settings, 2.try_into().unwrap()).unwrap();
    assert!(world.append(1.try_into().unwrap()).is_err());
    assert_eq!(world.snapshot().bodies.len(), 1);
    assert!(
        bevy_physics_bench::InitialScene::drop_batch(
            &Settings::default(),
            1.try_into().unwrap(),
            32768.try_into().unwrap()
        )
        .is_err()
    );
    let boxes: Settings = r#"{"scenario":"boxes"}"#.parse().unwrap();
    assert!(
        bevy_physics_bench::InitialScene::drop_batch(
            &boxes,
            1.try_into().unwrap(),
            32768.try_into().unwrap()
        )
        .is_err()
    );
}

#[test]
fn planar_growth_uses_the_same_bounded_lattice_without_depth() {
    let settings: Settings = r#"{"scenario":"balls","dimension":"2d"}"#.parse().unwrap();
    let batch = bevy_physics_bench::InitialScene::drop_batch(
        &settings,
        2.try_into().unwrap(),
        7.try_into().unwrap(),
    )
    .unwrap();
    assert_eq!(batch.bodies().len(), 2);
    assert_eq!(batch.bodies()[0].position().z, 0.0);
    assert_eq!(batch.bodies()[1].position().z, 0.0);
    assert!(batch.bodies()[1].position().y > batch.bodies()[0].position().y);
    assert!(matches!(
        batch.bodies()[0].shape(),
        bevy_physics_bench::Shape::Ball { radius: 0.5 }
    ));
    let mut world =
        Simulation::with_capacity(Engine::Avian, settings, 256.try_into().unwrap()).unwrap();
    world.append(2.try_into().unwrap()).unwrap();
    assert_eq!(world.snapshot().bodies.len(), 130);
    let mut world =
        Simulation::with_capacity(Engine::Rapier, settings, 256.try_into().unwrap()).unwrap();
    world.append(2.try_into().unwrap()).unwrap();
    assert_eq!(world.snapshot().bodies.len(), 130);
}

#[test]
fn kick_wakes_a_settled_body() {
    for engine in [Engine::Avian, Engine::Rapier, Engine::Jolt] {
        if engine == Engine::Jolt && !cfg!(feature = "jolt") {
            continue;
        }
        for dimension in ["2d", "3d"] {
            if engine == Engine::Jolt && dimension == "2d" {
                continue;
            }
            let settings: Settings = format!(
                r#"{{"scenario":"boxes","dimension":"{dimension}","count":1,"sleeping":"enabled"}}"#
            )
            .parse()
            .unwrap();
            let mut world = Simulation::new(engine, settings).unwrap();
            world.advance(StepCount::try_from(600).unwrap()).unwrap();
            assert!(world.snapshot().bodies[0].is_sleeping);
            world
                .kick(BodyKick::try_from((0, [0.0, 6.0, 0.0])).unwrap())
                .unwrap();
            world.advance(StepCount::try_from(10).unwrap()).unwrap();
            assert!(!world.snapshot().bodies[0].is_sleeping);
            assert!(world.snapshot().bodies[0].position[1] > 1.0);
        }
    }
}

#[test]
fn a_kick_moves_the_selected_body_in_both_cpu_dimensions() {
    for engine in [Engine::Avian, Engine::Rapier, Engine::Jolt] {
        for dimension in ["2d", "3d"] {
            if engine == Engine::Jolt && (dimension == "2d" || !cfg!(feature = "jolt")) {
                continue;
            }
            let settings: Settings = format!(
                r#"{{"dimension":"{dimension}","scenario":"boxes","count":1,"gravity":0}}"#
            )
            .parse()
            .unwrap();
            let mut simulation = Simulation::new(engine, settings).unwrap();
            let initial = simulation.snapshot();
            simulation
                .kick(BodyKick::try_from((0, [6.0, 0.0, 0.0])).unwrap())
                .unwrap();
            assert_eq!(simulation.snapshot().tick, 0);
            simulation
                .advance(StepCount::try_from(10).unwrap())
                .unwrap();
            let after = simulation.snapshot();
            assert!(
                after.bodies[0].position[0] > initial.bodies[0].position[0] + 0.9,
                "{engine:?} {dimension}"
            );
        }
    }
}

#[test]
fn growing_a_live_pile_retains_existing_poses_and_tick() {
    for engine in [Engine::Avian, Engine::Rapier, Engine::Jolt] {
        if engine == Engine::Jolt && !cfg!(feature = "jolt") {
            continue;
        }
        let settings: Settings = r#"{"scenario":"balls","count":1,"gravity":0}"#.parse().unwrap();
        let mut world = Simulation::with_capacity(engine, settings, 4.try_into().unwrap()).unwrap();
        world.advance(StepCount::try_from(10).unwrap()).unwrap();
        let before = world.snapshot();
        world.append(3.try_into().unwrap()).unwrap();
        let after = world.snapshot();
        assert_eq!(after.tick, 10);
        assert_eq!(after.bodies.len(), 4);
        assert_eq!(before.bodies[0].position, after.bodies[0].position);
        assert!(world.append(1.try_into().unwrap()).is_err());
        assert_eq!(world.snapshot().bodies.len(), 4);
        world
            .kick(BodyKick::try_from((3, [6.0, 0.0, 0.0])).unwrap())
            .unwrap();
        world.advance(StepCount::try_from(10).unwrap()).unwrap();
        let moved = world.snapshot();
        assert!(moved.bodies[3].position[0] > after.bodies[3].position[0] + 0.9);
        assert_eq!(moved.tick, 20);
    }
}

//! GPU tests distinguish unsupported profiles from failures to initialize real hardware.

use bevy_physics_bench::Settings;
use bevy_physics_bench_gpu::GpuSimulation;

#[test]
#[ignore = "requires a working GPU adapter"]
fn growing_floor_retains_bodies_beyond_the_fixed_arena() {
    futures::executor::block_on(async {
        let settings: Settings = r#"{"scenario":"boxes","count":1}"#.parse().unwrap();
        let capacity = 2.try_into().unwrap();
        let mut world = GpuSimulation::with_capacity(settings, capacity)
            .await
            .unwrap();
        assert_eq!(
            world.fingerprint(),
            bevy_physics_bench::SceneFingerprint::with_capacity(&settings, capacity).unwrap()
        );
        world
            .kick(bevy_physics_bench::BodyKick::try_from((0, [20.0, 0.0, 0.0])).unwrap())
            .await
            .unwrap();
        for _ in 0..180 {
            world.advance().await.unwrap();
        }
        let body = &world.snapshot().bodies[0];
        assert!(body.position[0] > 20.0, "{body:?}");
        assert!(body.position[1] >= -0.25, "{body:?}");
    });
}

#[test]
#[ignore = "requires a working GPU adapter"]
fn interactions_and_growth_preserve_the_existing_gpu_world() {
    futures::executor::block_on(async {
        for scenario in ["boxes", "balls"] {
            let settings: Settings =
                serde_json::json!({"scenario": scenario, "count": 1, "gravity": 0})
                    .to_string()
                    .parse()
                    .unwrap();
            let mut world = GpuSimulation::with_capacity(settings, 4.try_into().unwrap())
                .await
                .unwrap();
            let initial = world.snapshot();
            assert!(
                world
                    .kick(bevy_physics_bench::BodyKick::try_from((4, [1.0, 0.0, 0.0])).unwrap())
                    .await
                    .is_err()
            );
            assert_eq!(world.snapshot().tick, initial.tick);
            world
                .kick(bevy_physics_bench::BodyKick::try_from((0, [6.0, 0.0, 0.0])).unwrap())
                .await
                .unwrap();
            for _ in 0..10 {
                world.advance().await.unwrap();
            }
            assert!(world.snapshot().bodies[0].position[0] > initial.bodies[0].position[0] + 0.9);
            let before = world.snapshot();
            world.append(3.try_into().unwrap()).await.unwrap();
            assert_eq!(world.snapshot().bodies.len(), 4);
            assert_eq!(world.snapshot().tick, 10);
            assert_eq!(
                world.snapshot().bodies[0].position,
                before.bodies[0].position
            );
            assert!(world.append(1.try_into().unwrap()).await.is_err());
            world
                .kick(bevy_physics_bench::BodyKick::try_from((3, [1.0, 0.0, 0.0])).unwrap())
                .await
                .unwrap();
            world.advance().await.unwrap();
            assert_eq!(world.snapshot().tick, 11);
        }
    });
}

#[test]
#[ignore = "requires a GPU that supports a 717 MB storage buffer"]
fn ten_thousand_bodies_complete_on_a_capable_device() {
    futures::executor::block_on(async {
        let settings: Settings = r#"{"scenario":"boxes","count":10000}"#.parse().unwrap();
        let mut simulation = GpuSimulation::new(settings).await.unwrap();
        simulation.advance().await.unwrap();
        assert_eq!(simulation.snapshot().bodies.len(), 10000);
        assert_eq!(simulation.snapshot().tick, 1);
    });
}

#[test]
fn rejects_unsupported_profiles_before_requesting_a_device() {
    let settings: Settings = r#"{"dimension":"2d"}"#.parse().unwrap();
    let result = futures::executor::block_on(GpuSimulation::new(settings));
    assert!(matches!(
        result,
        Err(bevy_physics_bench::BenchError::Unsupported(_))
    ));
}

#[test]
fn growing_profiles_fail_before_requesting_a_device() {
    futures::executor::block_on(async {
        for json in [
            r#"{"scenario":"boxes","hz":30}"#,
            r#"{"scenario":"boxes","solver_effort":3}"#,
            r#"{"scenario":"pyramid"}"#,
        ] {
            assert!(matches!(
                GpuSimulation::with_capacity(json.parse().unwrap(), 256.try_into().unwrap()).await,
                Err(bevy_physics_bench::BenchError::Unsupported(_))
            ));
        }
        assert!(matches!(
            GpuSimulation::with_capacity(Settings::default(), 1.try_into().unwrap()).await,
            Err(bevy_physics_bench::BenchError::InvalidSetting(_))
        ));
    });
}

#[test]
#[ignore = "requires a working GPU adapter; run explicitly on the collection host"]
fn a_real_gpu_completes_steps_and_returns_finite_moving_poses() {
    futures::executor::block_on(async {
        let settings: Settings = r#"{"count":4,"solver_effort":4}"#.parse().unwrap();
        let mut simulation = GpuSimulation::new(settings).await.unwrap();
        let initial = simulation.snapshot();
        for _ in 0..30 {
            simulation.advance().await.unwrap();
        }
        let snapshot = simulation.snapshot();
        assert_eq!(snapshot.tick, 30);
        assert!(
            snapshot
                .bodies
                .iter()
                .all(|body| body.position.iter().all(|value| value.is_finite()))
        );
        assert!(snapshot.bodies[0].position[1] < initial.bodies[0].position[1]);
        assert!(simulation.last_readback().as_nanos() > 0);
    });
}

#[test]
#[ignore = "requires a working GPU adapter; run explicitly on the collection host"]
fn every_supported_scene_completes_with_matching_identity_and_metadata() {
    futures::executor::block_on(async {
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
            let settings: Settings = serde_json::json!({"scenario": scenario, "count": 12})
                .to_string()
                .parse()
                .unwrap();
            let expected = bevy_physics_bench::SceneFingerprint::of(&settings).unwrap();
            let mut simulation = GpuSimulation::new(settings).await.unwrap();
            assert_eq!(simulation.fingerprint(), expected);
            assert!(
                simulation.metadata()["name"]
                    .as_str()
                    .is_some_and(|name| !name.is_empty())
            );
            assert!(format!("{simulation:?}").contains("GpuSimulation"));
            for _ in 0..5 {
                simulation.advance().await.unwrap();
            }
            let snapshot = simulation.snapshot();
            assert_eq!(snapshot.tick, 5);
            assert_eq!(snapshot.bodies.len(), 12);
            assert!(snapshot.bodies.iter().all(|body| {
                body.position
                    .iter()
                    .chain(&body.rotation)
                    .all(|value| value.is_finite())
            }));
            if matches!(scenario, "joint-chain" | "ragdolls") {
                assert!(!snapshot.joints.is_empty());
            }
        }
    });
}

#[test]
#[ignore = "requires a working GPU adapter"]
fn static_gpu_bodies_reject_kicks_without_mutation() {
    futures::executor::block_on(async {
        let settings: Settings = r#"{"scenario":"static-field","count":12}"#.parse().unwrap();
        let mut world = GpuSimulation::new(settings).await.unwrap();
        let before = world.snapshot();
        let index = before
            .bodies
            .iter()
            .position(|body| body.kind == bevy_physics_bench::BodyKind::Static)
            .unwrap();
        let kick = bevy_physics_bench::BodyKick::try_from((index, [1.0, 0.0, 0.0])).unwrap();
        assert!(matches!(
            world.kick(kick).await,
            Err(bevy_physics_bench::BenchError::Unsupported(_))
        ));
        assert_eq!(
            world.snapshot().bodies[index].position,
            before.bodies[index].position
        );
        assert_eq!(world.snapshot().tick, 0);
    });
}

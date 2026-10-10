//! GPU tests distinguish unsupported profiles from failures to initialize real hardware.

use bevy_physics_bench::Settings;
use bevy_physics_bench_gpu::GpuSimulation;

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

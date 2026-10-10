//! Verify Jolt's native binding through the same scene and snapshot boundary.

#[cfg(all(feature = "jolt", not(target_arch = "wasm32")))]
#[test]
fn jolt_falls_and_rejects_unsupported_dimensions() {
    use bevy_physics_bench::{Dimension, Engine, Scenario, Settings, Simulation, StepCount};
    let mut world = Simulation::new(Engine::Jolt, r#"{"count":4}"#.parse().unwrap()).unwrap();
    let initial = world.snapshot();
    world.advance(StepCount::try_from(120).unwrap()).unwrap();
    let state = world.snapshot();
    assert!(state.bodies[0].position[1] < initial.bodies[0].position[1]);
    assert!(state.bodies.iter().all(|body| body.position[1] >= 0.45));
    assert!(
        Simulation::new(
            Engine::Jolt,
            Settings::for_scenario(Scenario::Boxes, Dimension::Two)
        )
        .is_err()
    );
}

#[cfg(all(feature = "jolt", not(target_arch = "wasm32")))]
#[test]
fn jolt_runs_the_complete_spatial_workload_catalog() {
    use bevy_physics_bench::{Engine, Settings, Simulation, StepCount};
    for scenario in [
        "boxes",
        "balls",
        "pyramid",
        "stacks",
        "static-field",
        "kinematics",
        "dominoes",
        "joint-chain",
        "ragdolls",
        "ccd",
        "queries",
        "characters",
    ] {
        let settings = format!(r#"{{"scenario":"{scenario}","count":24,"ccd":"continuous"}}"#)
            .parse::<Settings>()
            .unwrap();
        let mut world = Simulation::new(Engine::Jolt, settings).unwrap();
        world.advance(StepCount::try_from(120).unwrap()).unwrap();
        let state = world.snapshot();
        assert_eq!(state.bodies.len(), 24);
        assert!(
            state
                .bodies
                .iter()
                .all(|body| body.position.iter().all(|value| value.is_finite())),
            "{scenario}"
        );
        assert!(
            state.bodies.iter().all(|body| body.position[1] >= -0.25),
            "{scenario}: {state:?}"
        );
        if scenario == "queries" {
            assert_eq!(state.queries.hits, 12);
        }
    }
}

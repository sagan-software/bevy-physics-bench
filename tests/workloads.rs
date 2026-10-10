//! Workload behavior beyond an unconstrained falling pile.

use bevy_physics_bench::{BodyKind, Engine, Settings, Simulation, StepCount};

#[test]
fn every_workload_advances_in_both_dimensions() {
    for engine in [Engine::Avian, Engine::Rapier] {
        for dimension in ["2d", "3d"] {
            for scenario in [
                "stacks",
                "static-field",
                "kinematics",
                "dominoes",
                "joint-chain",
                "ragdolls",
                "ccd",
            ] {
                let configuration = format!(
                    r#"{{"scenario":"{scenario}","dimension":"{dimension}","count":24,"ccd":"continuous"}}"#
                );
                let settings = configuration.parse::<Settings>().unwrap();
                let mut simulation = Simulation::new(engine, settings).unwrap();
                simulation
                    .advance(StepCount::try_from(60).unwrap())
                    .unwrap();
                let snapshot = simulation.snapshot();
                assert_eq!(snapshot.bodies.len(), 24);
                assert!(
                    snapshot
                        .bodies
                        .iter()
                        .all(|body| body.position.iter().all(|value| value.is_finite()))
                );
                if dimension == "2d" {
                    assert!(snapshot.bodies.iter().all(|body| body.position[2] == 0.0));
                }
            }
        }
    }
}

#[test]
fn static_bodies_remain_fixed_and_kinematic_bodies_follow_the_driver() {
    for engine in [Engine::Avian, Engine::Rapier] {
        for scenario in ["static-field", "kinematics"] {
            let settings = format!(r#"{{"scenario":"{scenario}","count":16}}"#)
                .parse()
                .unwrap();
            let mut simulation = Simulation::new(engine, settings).unwrap();
            let before = simulation.snapshot();
            simulation
                .advance(StepCount::try_from(60).unwrap())
                .unwrap();
            let after = simulation.snapshot();
            for (initial, current) in before.bodies.iter().zip(&after.bodies) {
                if initial.kind == BodyKind::Static {
                    assert_eq!(initial.position, current.position);
                }
                if initial.kind == BodyKind::Kinematic {
                    assert_ne!(initial.position[0], current.position[0]);
                }
            }
        }
    }
}

#[test]
fn joint_chains_stay_connected_after_falling() {
    for engine in [Engine::Avian, Engine::Rapier] {
        let settings = r#"{"scenario":"joint-chain","count":8,"solver_effort":16}"#
            .parse()
            .unwrap();
        let mut simulation = Simulation::new(engine, settings).unwrap();
        simulation
            .advance(StepCount::try_from(180).unwrap())
            .unwrap();
        let snapshot = simulation.snapshot();
        assert_eq!(snapshot.joints.len(), 7);
        for joint in &snapshot.joints {
            let first = snapshot.bodies[joint[0]].position;
            let second = snapshot.bodies[joint[1]].position;
            let distance = first
                .iter()
                .zip(second)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f32>()
                .sqrt();
            assert!(distance < 1.0, "{engine:?} joint stretched to {distance} m");
        }
    }
}

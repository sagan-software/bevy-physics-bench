//! Public physics contract shared by native and browser callers.

use bevy_physics_bench::{Engine, Settings, Simulation, StepCount};

#[test]
fn both_engines_simulate_gravity_and_collide_with_the_floor() {
    for engine in [Engine::Avian, Engine::Rapier] {
        let mut simulation = Simulation::new(engine, Settings::default()).unwrap();
        let initial = simulation.snapshot();
        simulation
            .advance(StepCount::try_from(120).unwrap())
            .unwrap();
        let settled = simulation.snapshot();
        assert_eq!(settled.tick, 120);
        assert_eq!(initial.bodies.len(), settled.bodies.len());
        assert!(
            settled
                .bodies
                .iter()
                .all(|body| body.position[1].is_finite())
        );
        assert!(settled.bodies.iter().any(|body| body.position[1] < 1.0));
        assert!(settled.bodies.iter().all(|body| body.position[1] > -0.5));
    }
}

#[test]
fn enabled_sleeping_deactivates_settled_bodies_in_each_cpu_integration() {
    use bevy_physics_bench::{Dimension, Scenario, SettingsInput, Sleeping};
    for engine in [Engine::Avian, Engine::Rapier, Engine::Jolt] {
        for dimension in [Dimension::Two, Dimension::Three] {
            if engine == Engine::Jolt && (dimension == Dimension::Two || !cfg!(feature = "jolt")) {
                continue;
            }
            let defaults = SettingsInput::default();
            let settings = Settings::try_from(SettingsInput {
                scenario: Scenario::Boxes,
                dimension,
                count: 1.try_into().unwrap(),
                sleeping: Sleeping::Enabled,
                ..defaults
            })
            .unwrap();
            let mut world = Simulation::new(engine, settings).unwrap();
            world.advance(StepCount::try_from(600).unwrap()).unwrap();
            let snapshot = world.snapshot();
            assert!(snapshot.bodies[0].is_sleeping, "{engine:?} {dimension:?}");
            assert!(snapshot.bodies[0].position[1] > 0.4);
        }
    }
}

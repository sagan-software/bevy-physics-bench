//! Exercise spatial queries and character motion through the public simulation API.

use bevy_physics_bench::{
    BodyCount, BodyKind, Dimension, Engine, Scenario, Settings, SettingsInput, Simulation,
    StepCount,
};

#[test]
fn ray_queries_hit_the_shared_field_in_both_engines_and_dimensions() {
    for engine in [Engine::Avian, Engine::Rapier] {
        for dimension in [Dimension::Two, Dimension::Three] {
            let settings = Settings::try_from(SettingsInput {
                scenario: Scenario::Queries,
                dimension,
                count: BodyCount::try_from(16).unwrap(),
                ..SettingsInput::default()
            })
            .unwrap();
            let mut world = Simulation::new(engine, settings).unwrap();
            world.advance(StepCount::try_from(2).unwrap()).unwrap();
            let state = world.snapshot();
            assert_eq!(state.queries.rays, 16);
            assert_eq!(state.queries.hits, 8);
        }
    }
}

#[test]
fn characters_move_and_remain_above_the_floor() {
    for engine in [Engine::Avian, Engine::Rapier] {
        for dimension in [Dimension::Two, Dimension::Three] {
            let settings = Settings::try_from(SettingsInput {
                scenario: Scenario::Characters,
                dimension,
                count: BodyCount::try_from(8).unwrap(),
                ..SettingsInput::default()
            })
            .unwrap();
            let mut world = Simulation::new(engine, settings).unwrap();
            let initial = world.snapshot();
            world.advance(StepCount::try_from(120).unwrap()).unwrap();
            let final_state = world.snapshot();
            assert!(
                initial
                    .bodies
                    .iter()
                    .zip(&final_state.bodies)
                    .any(|(before, after)| before.kind == BodyKind::Kinematic
                        && (after.position[0] - before.position[0]).abs() > 0.1),
                "{engine:?} {dimension:?}"
            );
            for (before, after) in initial.bodies.iter().zip(final_state.bodies) {
                if before.kind == BodyKind::Kinematic {
                    assert!(
                        after.position[1] >= 0.3,
                        "{engine:?} {dimension:?} {after:?}"
                    );
                }
            }
        }
    }
}

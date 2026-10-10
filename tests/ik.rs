//! Keep animation work outside physics-engine rankings.

use bevy_physics_bench::{Dimension, Engine, Scenario, Settings, Simulation, StepCount};

#[test]
fn two_bone_ik_moves_targets_without_stretching_limbs() {
    for dimension in [Dimension::Two, Dimension::Three] {
        let settings = Settings::for_scenario(Scenario::InverseKinematics, dimension);
        assert!(Simulation::new(Engine::Avian, settings).is_err());
        let mut world = Simulation::new(Engine::Bevy, settings).unwrap();
        let initial = world.snapshot();
        world.advance(StepCount::try_from(30).unwrap()).unwrap();
        let state = world.snapshot();
        assert_ne!(initial.bodies[2].position, state.bodies[2].position);
        for pair in &state.joints {
            let left = state.bodies[pair[0]].position;
            let right = state.bodies[pair[1]].position;
            let length = left
                .iter()
                .zip(right)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f32>()
                .sqrt();
            assert!((length - 1.0).abs() < 0.0001, "{dimension:?}: {length}");
        }
    }
}

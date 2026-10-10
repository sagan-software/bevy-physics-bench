//! Validation through the same JSON boundary used by browser and native runs.

use bevy_physics_bench::{Dimension, Engine, Scenario, Settings, Simulation, StepCount};

#[test]
fn both_dimensions_and_seeded_scenes_use_real_engines() {
    for engine in [Engine::Avian, Engine::Rapier] {
        for dimension in [Dimension::Two, Dimension::Three] {
            for scenario in [Scenario::Boxes, Scenario::Balls, Scenario::Pyramid] {
                let settings = Settings::for_scenario(scenario, dimension);
                let mut world = Simulation::new(engine, settings).unwrap();
                let mut repeat = Simulation::new(engine, settings).unwrap();
                assert_eq!(world.snapshot(), repeat.snapshot());
                world.advance(StepCount::try_from(120).unwrap()).unwrap();
                let snapshot = world.snapshot();
                assert!(snapshot.bodies.iter().all(|body| body.position[1] >= -0.25));
                assert_ne!(snapshot, repeat.snapshot());
            }
        }
    }
}

#[test]
fn settings_reject_invalid_numbers_and_unknown_fields() {
    for invalid in [
        r#"{"count":0}"#,
        r#"{"count":32769}"#,
        r#"{"count":-1}"#,
        r#"{"count":1.5}"#,
        r#"{"gravity":-1}"#,
        r#"{"gravity":51}"#,
        r#"{"friction":-0.1}"#,
        r#"{"friction":2.1}"#,
        r#"{"restitution":-0.1}"#,
        r#"{"restitution":1.1}"#,
        r#"{"solver_effort":0}"#,
        r#"{"solver_effort":33}"#,
        r#"{"hz":59}"#,
        r#"{"sleeping":"maybe"}"#,
        r#"{"dimension":"4d"}"#,
        r#"{"scenario":"unknown"}"#,
        r#"{"ccd":"maybe"}"#,
        r#"{"seed":-1}"#,
        r#"{"extra":1}"#,
        r#"{"count":null}"#,
        r#"{"gravity":null}"#,
        "not json",
    ] {
        assert!(invalid.parse::<Settings>().is_err(), "accepted {invalid}");
    }
}

#[test]
fn settings_accept_and_preserve_valid_boundaries() {
    for valid in [
        r#"{}"#,
        r#"{"count":1}"#,
        r#"{"count":32768}"#,
        r#"{"gravity":0}"#,
        r#"{"gravity":50}"#,
        r#"{"friction":0}"#,
        r#"{"friction":2}"#,
        r#"{"restitution":0}"#,
        r#"{"restitution":1}"#,
        r#"{"solver_effort":1}"#,
        r#"{"solver_effort":32}"#,
        r#"{"hz":30}"#,
        r#"{"hz":60}"#,
        r#"{"hz":120}"#,
        r#"{"hz":240}"#,
        r#"{"seed":0}"#,
        r#"{"seed":4294967295}"#,
        r#"{"sleeping":"enabled","ccd":"continuous"}"#,
    ] {
        let parsed = valid.parse::<Settings>().unwrap();
        let wire = serde_json::to_string(&parsed).unwrap();
        assert_eq!(parsed, wire.parse::<Settings>().unwrap());
    }
}

#[test]
fn each_material_or_gravity_value_rounds_to_f32_before_rust_validation() {
    for field in ["gravity", "friction", "restitution"] {
        let source = format!(r#"{{"{field}":0.10000000000000001}}"#);
        let settings = source.parse::<Settings>().unwrap();
        let value = serde_json::to_value(settings).unwrap();
        assert_eq!(value[field].to_string(), "0.10000000149011612");
        let encoded = serde_json::to_string(&settings).unwrap();
        assert!(encoded.contains(&format!(r#""{field}":0.1"#)));
        assert_eq!(encoded.parse::<Settings>().unwrap(), settings);
    }
}

#[test]
fn rust_bounds_apply_after_decimal_input_rounds_to_f32() {
    for (field, accepted, rejected, expected) in [
        ("gravity", "50.0000001", "50.00001", 50.0),
        ("friction", "2.00000001", "2.000001", 2.0),
        ("restitution", "1.00000001", "1.000001", 1.0),
    ] {
        let source = format!(r#"{{"{field}":{accepted}}}"#);
        let settings = source.parse::<Settings>().unwrap();
        assert_eq!(serde_json::to_value(settings).unwrap()[field], expected);
        let source = format!(r#"{{"{field}":{rejected}}}"#);
        assert!(source.parse::<Settings>().is_err());
    }
}

#[test]
fn direct_construction_rejects_nonfinite_coefficients_and_step_bounds() {
    use bevy_physics_bench::{BenchError, SettingsInput};
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let base = SettingsInput::default();
        for input in [
            SettingsInput {
                gravity: value,
                ..base
            },
            SettingsInput {
                friction: value,
                ..base
            },
            SettingsInput {
                restitution: value,
                ..base
            },
        ] {
            assert!(matches!(
                Settings::try_from(input),
                Err(BenchError::InvalidSetting(_))
            ));
        }
    }
    for value in [0, 10001, u32::MAX] {
        assert!(matches!(
            StepCount::try_from(value),
            Err(BenchError::InvalidSetting(_))
        ));
    }
    for value in [1, 10000] {
        assert_eq!(StepCount::try_from(value).unwrap().get(), value);
    }
    for text in ["", "__proto__", "AVIAN"] {
        assert!(text.parse::<Engine>().is_err());
    }
    for text in ["", "4d", "3D"] {
        assert!(text.parse::<Dimension>().is_err());
    }
    for text in ["", "box", "Pyramid"] {
        assert!(text.parse::<Scenario>().is_err());
    }
}

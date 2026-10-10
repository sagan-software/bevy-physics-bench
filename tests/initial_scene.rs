//! External adapters borrow the same generated scene that the Bevy engines consume.

use bevy_physics_bench::{InitialScene, SceneFingerprint, Settings};

#[test]
fn exported_scene_preserves_geometry_velocities_and_joint_anchors() {
    let settings = r#"{"scenario":"joint-chain","count":3}"#.parse::<Settings>().unwrap();
    let scene = InitialScene::new(&settings);
    assert_eq!(scene.bodies().len(), 3);
    assert_eq!(scene.bodies()[1].joint().unwrap().parent(), 0);
    assert_eq!(scene.bodies()[1].linear_velocity().length(), 0.0);
    assert!(scene.floor_half_extents().x >= 20.0);
    let hash = SceneFingerprint::of(&settings).unwrap();
    assert_eq!(hash.to_string().parse::<SceneFingerprint>().unwrap(), hash);
    assert_eq!(hash, SceneFingerprint::of(&settings).unwrap());
    assert_ne!(hash, SceneFingerprint::of(&Settings::default()).unwrap());
    for invalid in [
        "",
        "unknown",
        &"A".repeat(64),
        &"f".repeat(63),
        &"f".repeat(65),
        &"g".repeat(64),
    ] {
        assert!(invalid.parse::<SceneFingerprint>().is_err());
    }
}

#[test]
fn adapter_accessors_preserve_the_complete_input_pose_and_constraint() {
    use bevy_physics_bench::{BodyKind, Shape};
    let scene = InitialScene::new(&r#"{"scenario":"joint-chain","count":2}"#.parse().unwrap());
    let parent = std::hint::black_box(&scene.bodies()[0]);
    let child = std::hint::black_box(&scene.bodies()[1]);
    assert_eq!(parent.kind(), BodyKind::Static);
    assert_eq!(child.kind(), BodyKind::Dynamic);
    assert_eq!(
        child.shape(),
        Shape::Box {
            half_extents: [0.3; 3]
        }
    );
    assert_eq!(child.rotation().to_array(), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(child.linear_velocity().to_array(), [0.0; 3]);
    assert_eq!(child.angular_velocity().to_array(), [0.0; 3]);
    let joint = child.joint().unwrap();
    assert_eq!(joint.parent(), 0);
    let parent_anchor = parent.position() + joint.parent_anchor();
    let child_anchor = child.position() + joint.child_anchor();
    assert!((parent_anchor - child_anchor).length() < 0.00001);
    let simulation =
        bevy_physics_bench::Simulation::new(bevy_physics_bench::Engine::Avian, Settings::default())
            .unwrap();
    assert!(format!("{simulation:?}").contains("tick: 0"));
}

//! Build metadata comes from the compiled core used by each collector.

#[test]
fn compiled_metadata_identifies_the_target_toolchain_and_compatible_versions() {
    let metadata = bevy_physics_bench::build_metadata();
    assert_eq!(metadata["versions"]["bevy"], "0.19.1");
    assert_eq!(metadata["versions"]["oxijolt"], "1.0.1+jolt-5.6.0");
    assert!(
        metadata["build"]["compiler"]
            .as_str()
            .unwrap()
            .starts_with("rustc ")
    );
    assert_eq!(metadata["target"], "x86_64-unknown-linux-gnu");
}

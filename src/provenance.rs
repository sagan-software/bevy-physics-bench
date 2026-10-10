//! Compiled build facts shared by native and browser result collectors.

/// Return the compiler, target, source revision, enabled profile, and
/// compatible engine versions.
/// Metadata describes the compiled core rather than guessing from the host
/// running it.
pub fn build_metadata() -> serde_json::Value {
    // Profile names describe compile-time execution features rather than host guesses.
    let profile = if cfg!(feature = "native-tuned") {
        "native-tuned"
    } else {
        "baseline"
    };
    // The core always includes both planar and spatial CPU integrations.
    let mut features = vec!["avian2d", "avian3d", "bevy_rapier2d", "bevy_rapier3d"];
    if cfg!(all(feature = "jolt", not(target_arch = "wasm32"))) {
        features.push("jolt");
    }
    // Parallel execution and SIMD are explicit features of the tuned native build.
    if cfg!(feature = "native-tuned") {
        features.extend(["parallel", "avian-simd", "rapier-simd8"]);
    }
    // Version fields describe the project compatibility set; each measurement names its engine.
    serde_json::json!({
        "target": env!("BENCH_TARGET"), "profile": profile,
        // Build-script facts capture the compiler, target flags, and enabled integrations.
        "build": { "compiler": env!("BENCH_COMPILER"), "rustflags": env!("BENCH_RUSTFLAGS"), "features": features },
        // Dependency versions are pinned as a mutually compatible Bevy integration set.
        "versions": { "bevy": "0.19.1", "avian": "0.7.0", "bevy_rapier": "0.36.0",
            "rapier": "0.35.0-glamx0.2", "oxijolt": "1.0.1+jolt-5.6.0", "jolt_js": "1.1.0", "nexus": "0.6.0",
            "revision": env!("BENCH_REVISION") },
    })
}

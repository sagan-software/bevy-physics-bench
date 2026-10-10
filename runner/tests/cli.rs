//! Native command-line output is the same result format consumed by the site.

#[test]
fn native_runner_emits_real_samples_and_machine_provenance() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_bench"))
        .args([
            "bench",
            "--count",
            "2",
            "--samples",
            "5",
            "--repeats",
            "1",
            "--warmup",
            "1",
            "--batch",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["environment"]["runtime"], "native");
    assert_eq!(value["versions"]["bevy"], "0.19.1");
    assert_eq!(
        value["measurements"].as_array().unwrap().len(),
        if cfg!(feature = "jolt") { 3 } else { 2 }
    );
    assert_eq!(
        value["environment"]["profile"],
        if cfg!(feature = "native-tuned") {
            "native-tuned"
        } else {
            "baseline"
        }
    );
    assert!(
        value["environment"]["target"]
            .as_str()
            .unwrap()
            .contains(std::env::consts::ARCH)
    );
    assert!(
        value["build"]["compiler"]
            .as_str()
            .unwrap()
            .starts_with("rustc ")
    );
    for measurement in value["measurements"].as_array().unwrap() {
        assert_eq!(measurement["samples_ms"].as_array().unwrap().len(), 5);
        assert!(
            measurement["samples_ms"]
                .as_array()
                .unwrap()
                .iter()
                .all(|sample| sample.as_f64().unwrap() > 0.0)
        );
        assert_eq!(measurement["finite"], true);
        assert_eq!(measurement["scene_fingerprint"].as_str().unwrap().len(), 64);
        assert_eq!(measurement["scope"], "bevy-step");
        assert_eq!(measurement["queries"]["rays"], 0);
        assert!(measurement["readback_ms"].is_null());
    }
}

#[test]
fn suite_covers_every_supported_engine_scene_and_dimension() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_bench"))
        .args([
            "suite",
            "--count",
            "2",
            "--samples",
            "5",
            "--repeats",
            "1",
            "--warmup",
            "1",
            "--batch",
            "1",
        ])
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{error}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["measurements"].as_array().unwrap().len(),
        if cfg!(feature = "jolt") { 62 } else { 50 }
    );
    assert!(
        value["measurements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["scope"] == "bevy-animation")
    );
}

#[test]
fn native_runner_rejects_invalid_counts_before_running() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_bench"))
        .args(["bench", "--count", "0"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[cfg(feature = "viewer")]
#[test]
fn native_viewer_renders_a_frame_without_a_display() {
    let output = std::env::temp_dir().join(format!("physics-view-{}.ppm", std::process::id()));
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_bench"))
        .args(["view", "--count", "8", "--frame"])
        .arg(&output)
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{error}");
    let data = std::fs::read(&output).unwrap();
    assert!(data.starts_with(b"P6\n960 640\n255\n"));
    assert!(data.windows(3).any(|pixel| pixel == [36, 118, 102]));
    std::fs::remove_file(output).unwrap();
}

#[test]
fn engine_filter_keeps_only_supported_suite_rows_and_rejects_empty_selection() {
    for (engine, expected) in [("avian", 24), ("rapier", 24), ("bevy", 2)] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_bench"))
            .args([
                "suite",
                "--engine",
                engine,
                "--count",
                "1",
                "--samples",
                "5",
                "--repeats",
                "1",
                "--warmup",
                "1",
                "--batch",
                "1",
            ])
            .output()
            .unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{error}");
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let measurements = value["measurements"].as_array().unwrap();
        assert_eq!(measurements.len(), expected);
        assert!(measurements.iter().all(|row| row["engine"] == engine));
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_bench"))
        .args(["bench", "--engine", "bevy", "--scenario", "boxes"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no supported engine"));
}

#[test]
fn explicit_settings_and_file_output_round_trip_and_labels_reject_controls() {
    let root = std::env::temp_dir().join(format!("physics-cli-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let settings = root.join("settings.json");
    let result = root.join("result.json");
    std::fs::write(&settings, r#"{"count":3,"seed":7,"friction":1.25}"#).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_bench"))
        .args([
            "bench",
            "--engine",
            "avian",
            "--samples",
            "5",
            "--repeats",
            "2",
            "--warmup",
            "1",
            "--batch",
            "1",
            "--settings",
        ])
        .arg(&settings)
        .arg("--output")
        .arg(&result)
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{error}");
    assert!(output.stdout.is_empty());
    let run: serde_json::Value = serde_json::from_slice(&std::fs::read(&result).unwrap()).unwrap();
    assert_eq!(run["measurements"][0]["settings"]["count"], 3);
    assert_eq!(run["measurements"][0]["settings"]["seed"], 7);
    assert_eq!(run["measurements"][0]["settings"]["friction"], 1.25);
    assert_eq!(
        run["measurements"][0]["samples_ms"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    for label in ["", "bad\nlabel", &"x".repeat(121)] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_bench"))
            .args(["bench", "--machine", label])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    std::fs::remove_file(settings).unwrap();
    std::fs::remove_file(result).unwrap();
    std::fs::remove_dir(root).unwrap();
}

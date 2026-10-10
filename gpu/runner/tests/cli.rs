//! GPU collection remains explicit and validates arguments before creating graphics resources.

#[test]
fn collector_rejects_zero_bodies_without_requesting_a_device() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_gpu-bench"))
        .args(["bench", "--count", "0"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

//! Capture provenance from the compiler and Git checkout before building an
//! artifact.
//!
//! Native and browser collectors embed these facts in every exported
//! result. Environment
//! overrides allow CI to supply its checked-out revision. Local builds
//! retain a dirty
//! suffix when source changes have not been committed, preventing their
//! publication.

use std::{
    io::{self, Write},
    process::Command,
};

/// Emit Cargo metadata as protocol output and preserve I/O failures instead
/// of hiding them.
fn main() -> io::Result<()> {
    let mut output = io::stdout().lock();
    // Rebuild when compiler flags, source files, or the local revision change.
    for name in ["BUILD_REVISION", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"] {
        emit(&mut output, "rerun-if-env-changed", name)?;
    }
    // Watch local source and Git changes that can alter embedded provenance.
    for path in [".git/HEAD", ".git/index", "src"] {
        emit(&mut output, "rerun-if-changed", path)?;
    }
    // TARGET describes the compiled artifact, including cross-compiled WebAssembly.
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    let compiler_path = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    // The compiler command identifies the actual selected toolchain, including nightly builds.
    let compiler = command_text(Command::new(compiler_path).arg("--version"));
    // Decode Cargo's separator while preserving the compiler flags themselves.
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS")
        .unwrap_or_default()
        .replace('\u{1f}', " ");
    // CI supplies its checked-out revision; local builds inspect their own tree.
    let revision = std::env::var("BUILD_REVISION").unwrap_or_else(|_| local_revision());
    // Cargo receives one independently encoded value for every provenance field.
    [
        ("TARGET", target),
        ("COMPILER", compiler),
        ("RUSTFLAGS", flags),
        ("REVISION", revision),
    ]
    .into_iter()
    .try_for_each(|(name, value)| emit(&mut output, "rustc-env", &format!("BENCH_{name}={value}")))
}

/// Read successful UTF-8 command output without treating an unavailable
/// command as a known value.
fn command_text(command: &mut Command) -> String {
    // A failed tool invocation leaves an explicit provenance gap in the resulting artifact.
    let Ok(output) = command.output() else {
        return "unknown".to_owned();
    };
    if !output.status.success() {
        return "unknown".to_owned();
    }
    String::from_utf8(output.stdout)
        .map_or_else(|_| "unknown".to_owned(), |text| text.trim().to_owned())
}

/// Preserve the commit identity and mark uncommitted changes for local
/// development artifacts.
fn local_revision() -> String {
    let revision = command_text(Command::new("git").args(["rev-parse", "HEAD"]));
    // Check worktree status separately from the commit lookup.
    let status = command_text(Command::new("git").args(["status", "--porcelain"]));
    // A failed status command cannot establish a clean tree, so retain the dirty marker.
    if status.is_empty() {
        revision
    } else {
        format!("{revision}-dirty")
    }
}

/// Write a complete Cargo protocol record using the required key and value
/// spelling.
fn emit(output: &mut impl Write, key: &str, value: &str) -> io::Result<()> {
    // Preserve Cargo's record spelling independently of human logging conventions.
    let record = format!("cargo:{key}={value}\n");
    // Propagate a failed protocol write so Cargo cannot accept incomplete metadata.
    output.write_all(record.as_bytes())
}

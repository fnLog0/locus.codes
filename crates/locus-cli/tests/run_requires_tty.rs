//! Integration test: `locus run` without a TTY prints a clear error.

use std::process::Command;

#[test]
fn run_without_tty_prints_helpful_error() {
    let bin = env!("CARGO_BIN_EXE_locus");
    let out = Command::new(bin)
        .arg("run")
        .output()
        .expect("run locus");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("interactive terminal") || stderr.contains("TTY"),
        "stderr should mention TTY; got: {}",
        stderr
    );
}

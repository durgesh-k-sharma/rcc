use std::process::Command;

/// Path to the compiled rcc binary (set by Cargo for integration tests).
const RCC_BIN: &str = env!("CARGO_BIN_EXE_rcc");

#[test]
fn test_help_exits_successfully() {
    let output = Command::new(RCC_BIN)
        .arg("--help")
        .output()
        .expect("failed to run rcc --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("Usage: rcc"),
        "help output should contain usage. Got: {combined}"
    );
}

#[test]
fn test_version_exits_successfully() {
    let output = Command::new(RCC_BIN)
        .arg("--version")
        .output()
        .expect("failed to run rcc --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rcc v"),
        "version output should contain version. Got: {stdout}"
    );
}

#[test]
fn test_no_input_file_returns_error() {
    let output = Command::new(RCC_BIN)
        .output()
        .expect("failed to run rcc");

    assert!(!output.status.success());
}

#[test]
fn test_nonexistent_file_returns_error() {
    let output = Command::new(RCC_BIN)
        .arg("nonexistent.c")
        .output()
        .expect("failed to run rcc nonexistent.c");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot open"),
        "error message should mention can't open file. Got: {stderr}"
    );
}

#[test]
fn test_unknown_flag_returns_error() {
    let output = Command::new(RCC_BIN)
        .arg("--bogus-flag")
        .output()
        .expect("failed to run rcc --bogus-flag");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown flag"),
        "error message should mention unknown flag. Got: {stderr}"
    );
}

#[test]
fn test_compile_valid_c_file_succeeds() {
    // Write a temporary C source file.
    let tmpdir = std::env::temp_dir().join(format!("rcc_test_{}", std::process::id()));
    std::fs::create_dir_all(&tmpdir).expect("failed to create temp dir");
    let src_path = tmpdir.join("test_valid.c");
    std::fs::write(&src_path, "int main() { return 42; }\n")
        .expect("failed to write test source file");

    let output = Command::new(RCC_BIN)
        .arg(src_path.to_str().unwrap())
        .output()
        .expect("failed to run rcc on valid.c");

    // For now, the stub just reads the file and reports — it exits 0.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("read"),
        "stub should report reading the file. Got: {stderr}"
    );

    // Clean up.
    let _ = std::fs::remove_dir_all(&tmpdir);
}

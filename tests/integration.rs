use std::path::Path;
use std::process::Command;

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gpkg-to-png"));
    assert!(stdout.contains("--bbox"));
    assert!(stdout.contains("--resolution"));
    assert!(stdout.contains("--scale"));
}

#[test]
fn test_missing_resolution_and_scale() {
    // Neither --resolution nor --scale is provided
    let output = Command::new("cargo")
        .args(["run", "--", "test.gpkg", "-f", "gpkg"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--resolution")
            || stderr.contains("--scale")
            || stderr.contains("must be provided"),
        "Expected error about missing resolution/scale, got: {}",
        stderr
    );
}

#[test]
fn test_mutually_exclusive_resolution_and_scale() {
    // Both --resolution and --scale are provided
    let output = Command::new("cargo")
        .args(["run", "--", "test.gpkg", "-f", "gpkg", "--resolution=0.001", "--scale=10"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "Expected error about mutually exclusive options, got: {}",
        stderr
    );
}

#[test]
fn test_invalid_bbox() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "test.gpkg",
            "-f",
            "gpkg",
            "--bbox",
            "invalid",
            "--resolution",
            "0.001",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("bbox") || stderr.contains("expected 4"));
}

#[test]
fn test_file_not_found() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "nonexistent.gpkg",
            "-f",
            "gpkg",
            "--bbox=-4.5,48.0,-4.0,48.5",
            "--resolution=0.001",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("File"));
}

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_real_gpkg_file() {
    // This test requires test.gpkg to be present
    if !Path::new("test.gpkg").exists() {
        eprintln!("Skipping: test.gpkg not found");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "test.gpkg",
            "-f",
            "gpkg",
            "--bbox=-4.8,48.2,-4.3,48.6",
            "--resolution=0.001",
            "-o",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());

    // Check that at least one PNG was created
    let png_files: Vec<_> = std::fs::read_dir(temp_dir.path())
        .expect("Failed to read temp dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "png"))
        .collect();

    assert!(!png_files.is_empty(), "No PNG files were created");
}

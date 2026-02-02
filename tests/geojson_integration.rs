use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore] // Requires test.geojson file
fn test_render_geojson_with_bbox() {
    let output_dir = PathBuf::from("target/test-output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let status = Command::new("cargo")
        .args([
            "run",
            "--",
            "test.geojson",
            "-f", "geojson",
            "--bbox", "5.166,43.381,5.168,43.383",
            "--resolution", "0.00001",
            "-o", output_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute command");

    assert!(status.success());

    let output_file = output_dir.join("test.png");
    assert!(output_file.exists());

    // Check that file is not empty
    let metadata = std::fs::metadata(&output_file).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
#[ignore]
fn test_render_geojson_with_custom_output_name() {
    let output_dir = PathBuf::from("target/test-output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let status = Command::new("cargo")
        .args([
            "run",
            "--",
            "test.geojson",
            "-f", "geojson",
            "--bbox", "5.166,43.381,5.168,43.383",
            "--resolution", "0.00001",
            "--output-name", "custom-name",
            "-o", output_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute command");

    assert!(status.success());

    let output_file = output_dir.join("custom-name.png");
    assert!(output_file.exists());
}

#[test]
#[ignore]
fn test_geojson_with_layer_option_fails() {
    let status = Command::new("cargo")
        .args([
            "run",
            "--",
            "test.geojson",
            "-f", "geojson",
            "--bbox", "5.166,43.381,5.168,43.383",
            "--resolution", "0.00001",
            "--layer", "test",
        ])
        .status()
        .expect("Failed to execute command");

    assert!(!status.success());
}

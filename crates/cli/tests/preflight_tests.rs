use std::path::PathBuf;
use std::process::Command;

fn get_bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_printproof3d")
}

fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[test]
fn test_no_model_and_no_gcode_fails() {
    let output = Command::new(get_bin_path())
        .arg("preflight")
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .output()
        .expect("failed to execute printproof3d binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Must provide either --model or --gcode"));
}

#[test]
fn test_both_model_and_gcode_fails() {
    let output = Command::new(get_bin_path())
        .arg("preflight")
        .arg("--model")
        .arg(workspace_path("fixtures/tetrahedron.stl"))
        .arg("--gcode")
        .arg(workspace_path("fixtures/safe_print.gcode"))
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .output()
        .expect("failed to execute printproof3d binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot provide both --model and --gcode"));
}

#[test]
fn test_stl_preflight_pass() {
    let output = Command::new(get_bin_path())
        .arg("preflight")
        .arg("--model")
        .arg(workspace_path("fixtures/tetrahedron.stl"))
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--material")
        .arg(workspace_path("profiles/pla.json"))
        .output()
        .expect("failed to execute printproof3d binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["status"], "pass");
}

#[test]
fn test_stl_preflight_fail() {
    let output = Command::new(get_bin_path())
        .arg("preflight")
        .arg("--model")
        .arg(workspace_path("fixtures/open_triangle.stl"))
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--material")
        .arg(workspace_path("profiles/pla.json"))
        .output()
        .expect("failed to execute printproof3d binary");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["status"], "fail");

    let issues = report["issues"].as_array().unwrap();
    let has_non_manifold = issues.iter().any(|i| i["id"] == "MESH_NOT_MANIFOLD");
    assert!(has_non_manifold);
}

#[test]
fn test_gcode_preflight_pass() {
    let output = Command::new(get_bin_path())
        .arg("preflight")
        .arg("--gcode")
        .arg(workspace_path("fixtures/safe_print.gcode"))
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .output()
        .expect("failed to execute printproof3d binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["status"], "pass");
}

#[test]
fn test_gcode_preflight_fail() {
    let output = Command::new(get_bin_path())
        .arg("preflight")
        .arg("--gcode")
        .arg(workspace_path("fixtures/unsafe_temp.gcode"))
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .output()
        .expect("failed to execute printproof3d binary");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["status"], "fail");
}

#[test]
fn test_simulator_preflight_pass() {
    let output = Command::new(get_bin_path())
        .arg("preflight")
        .arg("--model")
        .arg(workspace_path("fixtures/tetrahedron.stl"))
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--material")
        .arg(workspace_path("profiles/pla.json"))
        .arg("--simulator")
        .arg("prusalink")
        .output()
        .expect("failed to execute printproof3d binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["status"], "pass");
    assert!(report["sliced_settings_assumed"]["simulator_telemetry"].is_object());
}

#[test]
fn test_simulator_profile_mismatch_fails() {
    let output = Command::new(get_bin_path())
        .arg("preflight")
        .arg("--model")
        .arg(workspace_path("fixtures/tetrahedron.stl"))
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--material")
        .arg(workspace_path("profiles/pla.json"))
        .arg("--simulator")
        .arg("rrf")
        .output()
        .expect("failed to execute printproof3d binary");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["status"], "fail");

    let issues = report["issues"].as_array().unwrap();
    let has_conn_fail = issues
        .iter()
        .any(|i| i["id"] == "PRINTER_CONNECTION_FAILED");
    assert!(has_conn_fail);

    let conn_fail_issue = issues
        .iter()
        .find(|i| i["id"] == "PRINTER_CONNECTION_FAILED")
        .unwrap();
    let message = conn_fail_issue["message"].as_str().unwrap();
    assert!(message.contains("Protocol family mismatch"));
}

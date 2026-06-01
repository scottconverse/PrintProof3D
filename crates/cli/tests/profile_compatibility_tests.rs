use std::fs;
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

fn create_temp_test_dir(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("printproof3d_test_{}_{}", name, std::process::id()));
    if path.exists() {
        let _ = fs::remove_dir_all(&path);
    }
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn test_list_printers_text_and_json() {
    let temp_dir = create_temp_test_dir("list_printers");

    // 1. Test empty directory
    let output_empty = Command::new(get_bin_path())
        .arg("list-printers")
        .arg("--directory")
        .arg(&temp_dir)
        .output()
        .unwrap();
    assert!(output_empty.status.success());
    let stdout_empty = String::from_utf8_lossy(&output_empty.stdout);
    assert!(stdout_empty.contains("No printer profiles found in directory"));

    let output_empty_json = Command::new(get_bin_path())
        .arg("list-printers")
        .arg("--directory")
        .arg(&temp_dir)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output_empty_json.status.success());
    let stdout_empty_json = String::from_utf8_lossy(&output_empty_json.stdout);
    let empty_arr: serde_json::Value = serde_json::from_str(&stdout_empty_json).unwrap();
    assert!(empty_arr.is_array());
    assert_eq!(empty_arr.as_array().unwrap().len(), 0);

    // 2. Add profiles (one valid printer, one valid material, one malformed)
    let printer_path = temp_dir.join("printer1.json");
    fs::copy(workspace_path("profiles/prusa_mk4.json"), &printer_path).unwrap();

    let material_path = temp_dir.join("material1.json");
    fs::copy(workspace_path("profiles/pla.json"), &material_path).unwrap();

    let malformed_path = temp_dir.join("malformed.json");
    fs::write(&malformed_path, "{invalid_json}").unwrap();

    // Test text listing
    let output = Command::new(get_bin_path())
        .arg("list-printers")
        .arg("--directory")
        .arg(&temp_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Printer Profiles found in"));
    assert!(stdout.contains("Manufacturer: Prusa"));
    // Material and malformed profiles should be skipped gracefully
    assert!(!stdout.contains("Polylactic"));

    // Test JSON listing
    let output_json = Command::new(get_bin_path())
        .arg("list-printers")
        .arg("--directory")
        .arg(&temp_dir)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output_json.status.success());
    let stdout_json = String::from_utf8_lossy(&output_json.stdout);
    let arr: serde_json::Value = serde_json::from_str(&stdout_json).unwrap();
    assert_eq!(arr.as_array().unwrap().len(), 1);
    assert_eq!(arr[0]["manufacturer"], "Prusa");
    assert_eq!(arr[0]["protocol_family"], "prusa_link");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_list_materials_text_and_json() {
    let temp_dir = create_temp_test_dir("list_materials");

    // Write a material and printer profile
    let printer_path = temp_dir.join("printer1.json");
    fs::copy(workspace_path("profiles/prusa_mk4.json"), &printer_path).unwrap();

    let material_path = temp_dir.join("material1.json");
    fs::copy(workspace_path("profiles/pla.json"), &material_path).unwrap();

    // Test text listing
    let output = Command::new(get_bin_path())
        .arg("list-materials")
        .arg("--directory")
        .arg(&temp_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Material Profiles found in"));
    assert!(stdout.contains("Name: Polylactic Acid"));
    // Printer profile skipped
    assert!(!stdout.contains("Manufacturer: Prusa"));

    // Test JSON listing
    let output_json = Command::new(get_bin_path())
        .arg("list-materials")
        .arg("--directory")
        .arg(&temp_dir)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output_json.status.success());
    let stdout_json = String::from_utf8_lossy(&output_json.stdout);
    let arr: serde_json::Value = serde_json::from_str(&stdout_json).unwrap();
    assert_eq!(arr.as_array().unwrap().len(), 1);
    assert_eq!(arr[0]["name"], "Polylactic Acid");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_inspect_profile() {
    // 1. Inspect valid printer profile
    let output_printer = Command::new(get_bin_path())
        .arg("inspect-profile")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .output()
        .unwrap();
    assert!(output_printer.status.success());
    let stdout_printer = String::from_utf8_lossy(&output_printer.stdout);
    assert!(stdout_printer.contains("Profile Type    : Printer Profile"));
    assert!(stdout_printer.contains("Manufacturer    : Prusa"));

    // 2. Inspect valid material profile in JSON format
    let output_material = Command::new(get_bin_path())
        .arg("inspect-profile")
        .arg(workspace_path("profiles/pla.json"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output_material.status.success());
    let stdout_material = String::from_utf8_lossy(&output_material.stdout);
    let wrapped: serde_json::Value = serde_json::from_str(&stdout_material).unwrap();
    assert_eq!(wrapped["profile_type"], "material");
    assert_eq!(wrapped["profile"]["name"], "Polylactic Acid");

    // 3. Inspect missing file
    let output_missing = Command::new(get_bin_path())
        .arg("inspect-profile")
        .arg("non_existent_file.json")
        .output()
        .unwrap();
    assert!(!output_missing.status.success());
    let stderr_missing = String::from_utf8_lossy(&output_missing.stderr);
    assert!(
        stderr_missing.contains("Failed to read profile file")
            || stderr_missing.contains("Error: Failed to read profile file")
    );

    // 4. Inspect malformed file
    let temp_dir = create_temp_test_dir("inspect_fail");
    let malformed_path = temp_dir.join("malformed.json");
    fs::write(&malformed_path, "{invalid_json}").unwrap();

    let output_malformed = Command::new(get_bin_path())
        .arg("inspect-profile")
        .arg(&malformed_path)
        .output()
        .unwrap();
    assert!(!output_malformed.status.success());
    let stderr_malformed = String::from_utf8_lossy(&output_malformed.stderr);
    assert!(stderr_malformed.contains("Structurally invalid profile or malformed JSON file"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_validate_profiles() {
    let temp_dir = create_temp_test_dir("validate_profiles");

    // 1. Valid printer profile
    let output_valid_printer = Command::new(get_bin_path())
        .arg("validate-printer-profile")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .output()
        .unwrap();
    assert!(output_valid_printer.status.success());
    let stdout_valid_p = String::from_utf8_lossy(&output_valid_printer.stdout);
    assert!(stdout_valid_p.contains("is valid"));

    // 2. Structurally invalid printer profile
    let invalid_printer_path = temp_dir.join("invalid_printer.json");
    fs::copy(
        workspace_path("profiles/prusa_mk4.json"),
        &invalid_printer_path,
    )
    .unwrap();
    // Modify to make hotend temp unsafe (> 500)
    let content = fs::read_to_string(&invalid_printer_path).unwrap();
    let modified = content.replace("\"max_hotend_temp\": 300.0", "\"max_hotend_temp\": 600.0");
    fs::write(&invalid_printer_path, modified).unwrap();

    let output_invalid_printer = Command::new(get_bin_path())
        .arg("validate-printer-profile")
        .arg(&invalid_printer_path)
        .output()
        .unwrap();
    assert!(!output_invalid_printer.status.success());
    let stderr_invalid_p = String::from_utf8_lossy(&output_invalid_printer.stderr);
    assert!(stderr_invalid_p.contains("validation failed"));

    // 3. Valid material profile in JSON format
    let output_valid_material = Command::new(get_bin_path())
        .arg("validate-material-profile")
        .arg(workspace_path("profiles/pla.json"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output_valid_material.status.success());
    let stdout_valid_m = String::from_utf8_lossy(&output_valid_material.stdout);
    let val_m_res: serde_json::Value = serde_json::from_str(&stdout_valid_m).unwrap();
    assert_eq!(val_m_res["valid"], true);
    assert_eq!(val_m_res["type"], "material");

    // 4. Structurally invalid material profile
    let invalid_material_path = temp_dir.join("invalid_material.json");
    fs::copy(workspace_path("profiles/pla.json"), &invalid_material_path).unwrap();
    let m_content = fs::read_to_string(&invalid_material_path).unwrap();
    let m_modified = m_content.replace(
        "\"cooling_fan_speed_pct\": 100.0",
        "\"cooling_fan_speed_pct\": 150.0",
    );
    fs::write(&invalid_material_path, m_modified).unwrap();

    let output_invalid_material = Command::new(get_bin_path())
        .arg("validate-material-profile")
        .arg(&invalid_material_path)
        .output()
        .unwrap();
    assert!(!output_invalid_material.status.success());
    let stderr_invalid_m = String::from_utf8_lossy(&output_invalid_material.stderr);
    assert!(stderr_invalid_m.contains("validation failed"));

    // 5. Malformed JSON validation test
    let malformed_path = temp_dir.join("malformed.json");
    fs::write(&malformed_path, "{malformed_json}").unwrap();

    let output_malformed = Command::new(get_bin_path())
        .arg("validate-printer-profile")
        .arg(&malformed_path)
        .output()
        .unwrap();
    assert!(!output_malformed.status.success());
    let stderr_malformed = String::from_utf8_lossy(&output_malformed.stderr);
    assert!(stderr_malformed.contains("Failed to parse printer profile JSON"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_check_compatibility_profiles_and_files() {
    let temp_dir = create_temp_test_dir("check_compatibility");

    // 1. Mismatched Printer & Material compatibility test (Fail)
    // Low printer max temp vs high material temp
    let printer_path = temp_dir.join("low_temp_printer.json");
    fs::copy(workspace_path("profiles/prusa_mk4.json"), &printer_path).unwrap();
    let p_content = fs::read_to_string(&printer_path).unwrap();
    let p_modified = p_content.replace("\"max_hotend_temp\": 300.0", "\"max_hotend_temp\": 200.0");
    fs::write(&printer_path, p_modified).unwrap();

    let material_path = temp_dir.join("high_temp_material.json");
    fs::copy(workspace_path("profiles/pla.json"), &material_path).unwrap();
    let m_content = fs::read_to_string(&material_path).unwrap();
    let m_modified = m_content
        .replace("\"min_nozzle_temp\": 190.0", "\"min_nozzle_temp\": 210.0")
        .replace("\"max_nozzle_temp\": 220.0", "\"max_nozzle_temp\": 240.0");
    fs::write(&material_path, m_modified).unwrap();

    let output = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(&printer_path)
        .arg("--material")
        .arg(&material_path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Status   : Fail") || stdout.contains("Fail") || stdout.contains("fail")
    );
    assert!(stdout.contains("HOTEND_TEMP_INSUFFICIENT"));

    // 2. Enclosure warning compatibility test (Warning)
    let unenclosed_printer_path = temp_dir.join("unenclosed_printer.json");
    fs::copy(
        workspace_path("profiles/prusa_mk4.json"),
        &unenclosed_printer_path,
    )
    .unwrap();
    // Prusa MK4 is unenclosed by default

    let enclosure_material_path = temp_dir.join("enclosure_material.json");
    fs::copy(
        workspace_path("profiles/pla.json"),
        &enclosure_material_path,
    )
    .unwrap();
    let em_content = fs::read_to_string(&enclosure_material_path).unwrap();
    let em_modified = em_content.replace(
        "\"enclosure_recommended\": false",
        "\"enclosure_recommended\": true",
    );
    fs::write(&enclosure_material_path, em_modified).unwrap();

    let output_warning = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(&unenclosed_printer_path)
        .arg("--material")
        .arg(&enclosure_material_path)
        .output()
        .unwrap();
    // Advisory warnings (status: Warning) should not cause the command to exit with failure.
    // Confirming that the command exits successfully (exit code 0).
    assert!(output_warning.status.success());
    let stdout_warning = String::from_utf8_lossy(&output_warning.stdout);
    assert!(stdout_warning.contains("Status   : Warning"));
    assert!(stdout_warning.contains("ENCLOSURE_REQUIRED"));

    // 3. Model compatibility checks (Pass & Fail)
    // Pass model compatibility
    let output_model_pass = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--model")
        .arg(workspace_path("fixtures/tetrahedron.stl"))
        .output()
        .unwrap();
    assert!(output_model_pass.status.success());
    let stdout_model_pass = String::from_utf8_lossy(&output_model_pass.stdout);
    assert!(stdout_model_pass.contains("Status   : Pass"));

    // Fail model compatibility
    let output_model_fail = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--model")
        .arg(workspace_path("fixtures/open_triangle.stl"))
        .output()
        .unwrap();
    assert!(!output_model_fail.status.success());
    let stdout_model_fail = String::from_utf8_lossy(&output_model_fail.stdout);
    assert!(stdout_model_fail.contains("Status   : Fail"));
    assert!(stdout_model_fail.contains("MESH_NOT_MANIFOLD"));

    // 4. G-code compatibility checks (Pass & Fail)
    // Pass G-code compatibility
    let output_gcode_pass = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--gcode")
        .arg(workspace_path("fixtures/safe_print.gcode"))
        .output()
        .unwrap();
    assert!(output_gcode_pass.status.success());
    let stdout_gcode_pass = String::from_utf8_lossy(&output_gcode_pass.stdout);
    assert!(stdout_gcode_pass.contains("Status   : Pass"));

    // Fail G-code compatibility
    let output_gcode_fail = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--gcode")
        .arg(workspace_path("fixtures/unsafe_temp.gcode"))
        .output()
        .unwrap();
    assert!(!output_gcode_fail.status.success());
    let stdout_gcode_fail = String::from_utf8_lossy(&output_gcode_fail.stdout);
    assert!(stdout_gcode_fail.contains("Status   : Fail"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_unsupported_formats_fail() {
    let subcommands = &[
        "list-printers",
        "list-materials",
        "inspect-profile",
        "validate-printer-profile",
        "validate-material-profile",
        "check-compatibility",
    ];

    for sub in subcommands {
        let mut cmd = Command::new(get_bin_path());
        cmd.arg(sub);
        if *sub == "inspect-profile"
            || *sub == "validate-printer-profile"
            || *sub == "validate-material-profile"
        {
            cmd.arg(workspace_path("profiles/prusa_mk4.json"));
        } else if *sub == "check-compatibility" {
            cmd.arg("--printer")
                .arg(workspace_path("profiles/prusa_mk4.json"));
        }
        cmd.arg("--format").arg("invalid-format-type");

        let output = cmd.output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!(
            "Subcommand: {}, stderr: {}, status: {:?}",
            sub, stderr, output.status
        );
        assert!(
            !output.status.success(),
            "Subcommand {} should fail with invalid format",
            sub
        );
        assert!(
            stderr.contains("Unsupported format"),
            "Subcommand {} stderr does not contain 'Unsupported format'. Got: {}",
            sub,
            stderr
        );
    }
}

#[test]
fn test_docs_examples_run_successfully() {
    // Exact examples listed in docs:

    // 1. list-printers --format text
    let output = Command::new(get_bin_path())
        .arg("list-printers")
        .arg("--format")
        .arg("text")
        .output()
        .unwrap();
    assert!(output.status.success());

    // 2. list-printers --format json
    let output = Command::new(get_bin_path())
        .arg("list-printers")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());

    // 3. list-materials --format text
    let output = Command::new(get_bin_path())
        .arg("list-materials")
        .arg("--format")
        .arg("text")
        .output()
        .unwrap();
    assert!(output.status.success());

    // 4. list-materials --format json
    let output = Command::new(get_bin_path())
        .arg("list-materials")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());

    // 5. inspect-profile profiles/prusa_mk4.json
    let output = Command::new(get_bin_path())
        .arg("inspect-profile")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .output()
        .unwrap();
    assert!(output.status.success());

    // 6. inspect-profile profiles/pla.json --format json
    let output = Command::new(get_bin_path())
        .arg("inspect-profile")
        .arg(workspace_path("profiles/pla.json"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());

    // 7. validate-printer-profile profiles/prusa_mk4.json
    let output = Command::new(get_bin_path())
        .arg("validate-printer-profile")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .output()
        .unwrap();
    assert!(output.status.success());

    // 8. validate-material-profile profiles/pla.json --format json
    let output = Command::new(get_bin_path())
        .arg("validate-material-profile")
        .arg(workspace_path("profiles/pla.json"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());

    // 9. check-compatibility --printer profiles/prusa_mk4.json --material profiles/pla.json
    let output = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--material")
        .arg(workspace_path("profiles/pla.json"))
        .output()
        .unwrap();
    assert!(output.status.success());

    // 10. check-compatibility --printer profiles/prusa_mk4.json --model fixtures/tetrahedron.stl
    let output = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--model")
        .arg(workspace_path("fixtures/tetrahedron.stl"))
        .output()
        .unwrap();
    assert!(output.status.success());

    // 11. check-compatibility --printer profiles/prusa_mk4.json --gcode fixtures/safe_print.gcode
    let output = Command::new(get_bin_path())
        .arg("check-compatibility")
        .arg("--printer")
        .arg(workspace_path("profiles/prusa_mk4.json"))
        .arg("--gcode")
        .arg(workspace_path("fixtures/safe_print.gcode"))
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_stage3_new_commands() {
    let temp_dir = tempfile::tempdir().unwrap();

    // 1. generate-printer-profile
    let printer_path = temp_dir.path().join("gen_printer.json");
    let output = Command::new(get_bin_path())
        .arg("generate-printer-profile")
        .arg("--output")
        .arg(&printer_path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(printer_path.exists());
    let printer_content = fs::read_to_string(&printer_path).unwrap();
    let printer_val: serde_json::Value = serde_json::from_str(&printer_content).unwrap();
    assert_eq!(printer_val["manufacturer"], "TemplateManufacturer");

    // 2. generate-material-profile
    let material_path = temp_dir.path().join("gen_material.json");
    let output = Command::new(get_bin_path())
        .arg("generate-material-profile")
        .arg("--output")
        .arg(&material_path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(material_path.exists());
    let material_content = fs::read_to_string(&material_path).unwrap();
    let material_val: serde_json::Value = serde_json::from_str(&material_content).unwrap();
    assert_eq!(material_val["name"], "Template PLA");

    // 3. validate-profile-directory (on temp_dir containing the two generated profiles)
    let output = Command::new(get_bin_path())
        .arg("validate-profile-directory")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout_val = String::from_utf8_lossy(&output.stdout);
    let results: serde_json::Value = serde_json::from_str(&stdout_val).unwrap();
    assert!(results.is_array());
    let arr = results.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert!(arr
        .iter()
        .any(|item| item["type"] == "printer" && item["valid"] == true));
    assert!(arr
        .iter()
        .any(|item| item["type"] == "material" && item["valid"] == true));

    // 4. validate-profile-directory in text mode
    let output = Command::new(get_bin_path())
        .arg("validate-profile-directory")
        .arg(temp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout_val_text = String::from_utf8_lossy(&output.stdout);
    assert!(stdout_val_text.contains("Printer profile"));
    assert!(stdout_val_text.contains("is valid"));
}

#[test]
fn test_validate_profile_directory_defaults_and_options() {
    // 1. No directory defaults to "profiles"
    let output = Command::new(get_bin_path())
        .arg("validate-profile-directory")
        .current_dir(workspace_path(""))
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("prusa_mk4.json") && stdout.contains("is valid"));

    // 2. Using option --directory profiles
    let output = Command::new(get_bin_path())
        .arg("validate-profile-directory")
        .arg("--directory")
        .arg("profiles")
        .current_dir(workspace_path(""))
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("prusa_mk4.json") && stdout.contains("is valid"));

    // 3. Using option -d profiles
    let output = Command::new(get_bin_path())
        .arg("validate-profile-directory")
        .arg("-d")
        .arg("profiles")
        .current_dir(workspace_path(""))
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_generate_printer_profile_contract() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("printer_test.json");

    let output = Command::new(get_bin_path())
        .arg("generate-printer-profile")
        .arg("--output")
        .arg(&file_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(file_path.exists());
    let content = std::fs::read_to_string(&file_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(val["manufacturer"], "TemplateManufacturer");
    assert_eq!(val["model"], "TemplateModel");

    // Check conflict when using format parameter (it is Option B, format option doesn't exist)
    let output = Command::new(get_bin_path())
        .arg("generate-printer-profile")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_generate_material_profile_contract() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("material_test.json");

    let output = Command::new(get_bin_path())
        .arg("generate-material-profile")
        .arg("--output")
        .arg(&file_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(file_path.exists());
    let content = std::fs::read_to_string(&file_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(val["name"], "Template PLA");

    // Check conflict when using format parameter
    let output = Command::new(get_bin_path())
        .arg("generate-material-profile")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

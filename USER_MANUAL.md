# PrintProof3D: Developer & Integration Manual

Welcome to the **PrintProof3D** Developer and Integration Manual. PrintProof3D is a modular, type-safe printability analysis and printer management engine written in Rust. It helps you run geometric validation (STL) and firmware/path limits audits (G-code) against specific hardware and material constraints *before* sending jobs to physical 3D printers.

This manual walks you through configuring profiles, running validation jobs via the command line, writing custom validation rules as sandboxed WebAssembly plugins, and integrating the engine into your services via the REST API or the Model Context Protocol (MCP) server.

---

## Table of Contents
1. [Understanding PrintProof3D](#1-understanding-printproof3d)
2. [Configuring Machine & Filament Profiles](#2-configuring-machine--filament-profiles)
3. [Running Validations via the CLI](#3-running-validations-via-the-cli)
4. [Tutorial: Writing a Custom WASM Validation Plugin](#4-tutorial-writing-a-custom-wasm-validation-plugin)
5. [Integrating the REST API & MCP Server](#5-integrating-the-rest-api--mcp-server)
6. [Testing Custom Printer Adapters for Compliance](#6-testing-custom-printer-adapters-for-compliance)

---

## 1. Understanding PrintProof3D

PrintProof3D operates on a **defense-in-depth** model for print safety. It separates validation (mesh checks, static G-code analysis) from printer control (sending commands, monitoring telemetry).

```
 [ 3D STL Model ] ──► ( 1. Geometry Parser: checks manifold, dimensions, overhangs )
                               │
 [ Sliced G-Code ] ──► ( 2. Path Audit: checks thermal limits, axis bounds )
                               │
                               ▼
            ( 3. WASM Plugin Sandbox: runs custom compliance rules )
                               │
                               ▼
                     [ Validation Report ]
```

By compiling custom checks to WebAssembly (WASM), developers can write custom compliance rules (e.g., verifying boundary margins, preventing specific patterns, auditing filament weights) that run inside a secure sandbox with zero direct system access.

---

## 2. Configuring Machine & Filament Profiles

To validate prints, you must define the target environment using two configuration formats: **Printer Profiles** and **Material Profiles**.

### 2.1 Printer Profile (`.json`)
This profile defines the physical limits and capabilities of your 3D printer.

```json
{
  "manufacturer": "Prusa",
  "model": "MK4",
  "protocol_family": "prusa_link",
  "build_volume": {
    "type": "rectangular",
    "x": 250.0,
    "y": 210.0,
    "z": 220.0
  },
  "bed_shape": "rectangular",
  "nozzle_diameters": [0.25, 0.4, 0.6, 0.8],
  "default_nozzle_diameter": 0.4,
  "min_layer_height": 0.05,
  "max_layer_height": 0.3,
  "max_hotend_temp": 300.0,
  "max_bed_temp": 120.0,
  "has_enclosure": false,
  "supports_mmu": true,
  "firmware_flavor": "prusa",
  "supported_file_types": ["gcode", "bgcode"],
  "supports_direct_upload": true,
  "supports_pause_resume": true,
  "supports_cancel": true,
  "supports_job_progress": true,
  "supports_webcam": false,
  "supports_chamber_temp": false,
  "known_quirks": ["long_heatup"],
  "unsafe_commands": ["M500"],
  "filename_restrictions": null
}
```

#### Detailed Key Explanations:
* `build_volume`: Specifies dimension limits. Can be `"rectangular"` (requires `x`, `y`, `z` bounds) or `"cylindrical"` (requires `diameter` and `z` height).
* `unsafe_commands`: A list of G-code instructions that are blacklisted (e.g., `M500` to prevent write wear on EEPROM).
* `filename_restrictions`: A regex pattern used to reject uploaded file names that are non-compliant or malicious.

### 2.2 Material Profile (`.json`)
This profile describes the thermal requirements and risks associated with your filament.

```json
{
  "name": "Polylactic Acid",
  "abbreviations": ["PLA"],
  "min_nozzle_temp": 190.0,
  "max_nozzle_temp": 220.0,
  "min_bed_temp": 50.0,
  "max_bed_temp": 60.0,
  "cooling_fan_speed_pct": 100.0,
  "warp_risk": "low",
  "bridge_difficulty": "low",
  "overhang_difficulty": "low",
  "enclosure_recommended": false,
  "dryness_sensitive": false,
  "bed_adhesion_notes": "Requires clean PEI sheet",
  "min_feature_size_mm": 0.4
}
```

---

## 3. Running Validations via the CLI

PrintProof3D provides a fast command line utility (`printproof3d`) for local execution and integration with shell scripts.

### 3.1 STL Model Validation
Verify that a raw 3D mesh is watertight, manifold, and fits within the build volume:
```bash
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

### 3.2 G-Code Validation
Audit sliced G-code coordinates, movement boundaries, and hotend/bed heatup thresholds:
```bash
printproof3d validate-gcode \
  --gcode fixtures/safe_print.gcode \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

### 3.3 Output Format
The CLI returns a JSON-formatted `ValidationReport` to stdout. If warnings or critical blocker/critical issues are detected, the CLI exits with code `1`. Otherwise, it returns `0`.

---

## 4. Tutorial: Writing a Custom WASM Validation Plugin

This tutorial guides you through creating a sandboxed plugin to enforce a custom rule: **flagging a warning if the printed model's volume is too small**.

### Step 1: Create a new library crate
Initialize a new Rust library outside the printproof3d tree or inside your development workspace:
```bash
cargo new --lib volume-check-plugin
cd volume-check-plugin
```

### Step 2: Configure `Cargo.toml`
Set the crate-type to `"cdylib"` (C-compatible dynamic library) and add the `printproof3d-core` and `printproof3d-plugins` paths as dependencies:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
printproof3d-core = { path = "../PrintProof3D/crates/core" }
printproof3d-plugins = { path = "../PrintProof3D/crates/plugins" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Step 3: Implement your validation rule
In `src/lib.rs`, import the required types, write a rule function, and call the `export_validation_plugin!` macro to wire up memory allocation and serialization exports:

```rust
use printproof3d_core::BuildVolume;
use printproof3d_plugins::{
    export_validation_plugin, ValidationReport, ValidationIssue, IssueSeverity, ValidationStatus
};

fn enforce_minimum_volume(report: &mut ValidationReport) {
    // 1. Calculate the bounding box volume of the model
    let volume = match &report.model.bounding_box {
        BuildVolume::Rectangular { x, y, z } => x * y * z,
        BuildVolume::Cylindrical { diameter, z } => {
            let radius = diameter / 2.0;
            std::f32::consts::PI * radius * radius * z
        }
    };

    // 2. Enforce a threshold (e.g. warning if volume is under 1000 mm³)
    if volume < 1000.0 {
        report.issues.push(ValidationIssue {
            id: "VOLUME_TOO_SMALL".to_string(),
            severity: IssueSeverity::Minor,
            message: format!("Model volume ({:.2} mm³) is under the 1000 mm³ warning limit.", volume),
            location: None,
            suggested_fixes: vec!["Scale up the model in the slicer before printing.".to_string()],
        });

        // Elevate report status if it was previously passing
        if report.status == ValidationStatus::Pass {
            report.status = ValidationStatus::Warning;
        }
    }
}

// Export the WASM interface for our plugin loader
export_validation_plugin!(enforce_minimum_volume);
```

### Step 4: Compile to WebAssembly
Run the compiler specifying the target `wasm32-unknown-unknown`:
```bash
cargo build --target wasm32-unknown-unknown --release
```

### Step 5: Execute validation with your plugin
Run the model validation command, passing your compiled plugin file to the `--plugin` argument:
```bash
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json \
  --plugin target/wasm32-unknown-unknown/release/volume_check_plugin.wasm
```
Your report will now include the custom warning under the `issues` list!

---

## 5. Integrating the REST API & MCP Server

PrintProof3D is designed to integrate into web systems, newsrooms, and agentic workflows.

### 5.1 REST API Integration
The embedded web server uses Axum and acts as a local validation microservice. Start the service:
```bash
cargo run --package printproof3d-rest
```

To validate files, perform a multipart/form-data `POST` request to `/validate/model` or `/validate/gcode` passing the file payload and the profile files. Secure endpoints require passing your configured Bearer auth token in the `Authorization` header.

### 5.2 Model Context Protocol (MCP)
AI development tools (like Cursor, Claude Desktop, etc.) can hook into the printproof3d MCP server over standard I/O:
```bash
printproof3d mcp
```
The server exposes tools (`validate_model_printability`, `validate_gcode`, `list_printer_profiles`, and `explain_validation_report`) which allows agents to audit, review, and explain G-code path anomalies and mesh integrity issues in plain language.

---

## 6. Testing Custom Printer Adapters for Compliance

If you are developing a custom connection adapter (e.g. supporting a new network protocol or firmware interface), you can verify its compliance using the automated conformance test suite in `printproof3d-sdk`.

1. Implement the `PrinterAdapter` trait on your client.
2. In a test block, pass your client instance to `run_conformance_tests`:

```rust
use printproof3d_adapters::PrinterAdapter;
use printproof3d_sdk::run_conformance_tests;

#[tokio::test]
async fn verify_my_custom_client() {
    let mut my_client = MyAdapterClient::new("192.168.1.50");
    let result = run_conformance_tests(&mut my_client).await;
    assert!(result.is_ok(), "Compliance failed: {:?}", result);
}
```
The compliance test suite verifies that connection states, telemetry fetches, pause/resume commands, and cancellations execute reliably and return appropriate errors under fault simulation.

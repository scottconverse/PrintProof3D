# PrintProof3D User Manual & Integration Guide

This guide describes how to configure print profiles, run CLI validations, spin up the local microservices, write WASM plugins, and verify adapter compliance.

---

## 1. Profiles Schema Definitions

PrintProof3D evaluates suitability using two JSON structures: **Printer Profiles** and **Material Profiles**.

### 1.1 Printer Profile (`profiles/prusa_mk4.json`)
Defines physical limits, kinematics, and communication capabilities of the machine:

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

### 1.2 Material Profile (`profiles/pla.json`)
Defines the chemical and thermal traits of the printing filament:

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

## 2. Command Line Interface (CLI)

PrintProof3D includes a command-line tool `printproof3d`.

### 2.1 Model Mesh Geometric Validation
Verify an STL file's topology, dimensions, and overhang structures:
```bash
printproof3d validate-model \
  -m fixtures/tetrahedron.stl \
  -p profiles/prusa_mk4.json \
  -a profiles/pla.json
```

### 2.2 Sliced G-code Verification
Inspect G-code files statically to check coordinate boundary compliance and temperature sanity checks:
```bash
printproof3d validate-gcode \
  -g fixtures/safe_print.gcode \
  -p profiles/prusa_mk4.json \
  -a profiles/pla.json
```

---

## 3. Sandboxed WASM Plugins (Custom Rules)

PrintProof3D supports extending validation using WebAssembly plugins.

### 3.1 Writing a Custom Validation Rule Crate
To write a plugin, set up a standard Rust library with `crate-type = ["cdylib"]` and depend on `printproof3d-plugins`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
printproof3d-plugins = { path = "../path_to/crates/plugins" }
```

In `src/lib.rs`, implement a validation function and use the `export_validation_plugin!` macro:

```rust
use printproof3d_plugins::{
    export_validation_plugin, ValidationReport, ValidationIssue, IssueSeverity, ValidationStatus
};

fn my_custom_rules(report: &mut ValidationReport) {
    // If the printbed shape is rectangular, ensure the model does not exceed a safety margin
    if report.model.bounding_box.max_x() > 200.0 {
        report.issues.push(ValidationIssue {
            id: "BED_MARGIN_EXCEEDED".to_string(),
            severity: IssueSeverity::Major,
            message: "Model exceeds 200mm safety margin on X axis.".to_string(),
            location: None,
            suggested_fixes: vec!["Center the model or scale it down.".to_string()],
        });
        report.status = ValidationStatus::Fail;
    }
}

export_validation_plugin!(my_custom_rules);
```

### 3.2 Compiling & Executing the Plugin
1. Build the library targeting WebAssembly:
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```
2. Execute validation loading the plugin:
   ```bash
   printproof3d validate-model \
     -m fixtures/tetrahedron.stl \
     -p profiles/prusa_mk4.json \
     -a profiles/pla.json \
     --plugin target/wasm32-unknown-unknown/release/my_plugin.wasm
   ```

---

## 4. Model Context Protocol (MCP) Server

PrintProof3D runs an MCP JSON-RPC 2.0 server over stdin/stdout. AI agents can pair to it to run model or G-code audits:

```bash
printproof3d mcp
```

### Supported Tools:
* `validate_model_printability`: Run geometry audits.
* `validate_gcode`: Audits movements and thermal windows.
* `list_printer_profiles`: Retrieves default profiles in the registry.
* `explain_validation_report`: Produces a text summary of issues.

---

## 5. REST API Services

The embedded HTTP server exposes endpoints on port `3000` with Bearer auth:

```bash
cargo run --package printproof3d-rest
```

### Key API Endpoints:
- `GET /profiles/printers` — Lists all default printer JSON profiles.
- `POST /validate/model` (Multipart) — Performs mesh audit. Exposes `model` (file), `printer` (JSON file), and `material` (JSON file).
- `POST /validate/gcode` (Multipart) — Performs G-code audit. Exposes `gcode` (file), `printer` (JSON file), and optional `material`.

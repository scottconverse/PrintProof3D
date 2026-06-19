# PrintProof3D Agent Integration Guide

Welcome, agent! This guide explains how to integrate and use the `PrintProof3D` compatibility and printability validation harness inside developer software like **KimCad**.

---

## 1. Introduction & Scope Limitations

### What PrintProof3D Is For
PrintProof3D provides an automated software validation harness. It assesses 3D models (STL files) and sliced G-code against safety constraints and configurations defined in Printer and Material profiles. It also provides simulated twin mock interfaces for printer protocol adapters (Bambu Lab MQTT/FTP, Moonraker/Klipper, OctoPrint, PrusaLink, RepRapFirmware, and Marlin Serial).

### What It Does NOT Claim (Hard Simulator-Only Limit)
> [!WARNING]
> **Do not claim real printer safety certification or real-world hardware validation.**
> All printer communication testing is done against simulator twin mocks. Passing tests proves client-side protocol conformance only, not real printer physical safety or actual hardware readiness.

---

## 2. First Command to Run (Health Check)

To verify that the workspace is fully functional, run the health-check runner from the project root:

```powershell
python devtools/agent_health_check.py
```

This script will verify formatting, run clippy lints, execute the full cargo test suite, compile the CLI binary, and run model & G-code validation smoke tests.

---

## 3. Profiles (Invariants & JSON Structures)

Validations require a **Printer Profile** and an optional **Material Profile** formatted as JSON.

### Printer Profile JSON
Stored in [profiles/prusa_mk4.json](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/profiles/prusa_mk4.json).
Key properties:
- `build_volume`: Specifies dimension limits (`x`, `y`, `z`).
- `max_hotend_temp`: Hotend temperature threshold.
- `max_bed_temp`: Bed temperature threshold.
- `protocol_family`: Protocol identifier (e.g. `PrusaLink`, `Klipper`, `OctoPrint`, etc.).

### Material Profile JSON
Stored in [profiles/pla.json](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/profiles/pla.json).
Key properties:
- `min_nozzle_temp` / `max_nozzle_temp`: Safety boundaries for hotend.
- `min_bed_temp` / `max_bed_temp`: Safety boundaries for bed.

---

## 4. Integration Paths & APIs

PrintProof3D supports four integration channels:

### A. CLI Integration
Compile the CLI tool using `cargo build --release` or `cargo build`.
The binary is located at `target/debug/printproof3d.exe` (Windows) or `target/debug/printproof3d` (Unix).

#### Model Validation Command:
```powershell
target/debug/printproof3d.exe validate-model --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json
```

#### G-code Validation Command:
```powershell
target/debug/printproof3d.exe validate-gcode --gcode fixtures/safe_print.gcode --printer profiles/prusa_mk4.json --material profiles/pla.json
```

### B. REST API Integration
Run the REST service by launching the `printproof3d-rest` binary (listening on `127.0.0.1:3000`).
All endpoints require a Bearer token header: `Authorization: Bearer <token>`. The token is read from the `PRINTPROOF3D_API_TOKEN` environment variable. If `PRINTPROOF3D_API_TOKEN` is not configured, the REST daemon generates a secure ephemeral startup token and prints it to the console (e.g., `[PrintProof3D API] Token is not configured. Ephemeral token generated: <token>`). Use this token for authorization.

#### POST `/validate/model`
Multipart upload:
- `model`: Binary STL file.
- `printer`: Printer profile JSON file.
- `material`: Material profile JSON file.

#### POST `/validate/gcode`
Multipart upload:
- `gcode`: Sliced G-code file.
- `printer`: Printer profile JSON file.
- `material`: Material profile JSON file (optional).

### C. Model Context Protocol (MCP) Integration
Launch the MCP JSON-RPC server over stdin/stdout:
```powershell
target/debug/printproof3d.exe mcp
```
Supported MCP tools:
- `validate_model_printability`: Expects `model_path`, `printer_profile_path`, `material_profile_path`.
- `validate_gcode`: Expects `gcode_path`, `printer_profile_path`, `material_profile_path` (optional).
- `list_printer_profiles`: Lists available printer profiles.
- `explain_validation_report`: Provides a natural-language description of a report JSON.

### D. SDK Conformance & Simulator Mocks
Crate `printproof3d-sdk` exposes test twin simulators for all adapter testing.
Example:
```rust
use printproof3d_sdk::mocks::RrfMockServer;
use printproof3d_adapters::rrf::RrfAdapter;

// Start twin simulator on dynamic ephemeral port (0)
let server = RrfMockServer::start();
let config = PrinterConnectionConfig {
    name: "RRF simulator".to_string(),
    base_url: Some(server.get_url()),
    // ...
};
let mut adapter = RrfAdapter::new(profile, config);
adapter.connect().await.unwrap();
let telemetry = adapter.get_status().await.unwrap();
assert_eq!(telemetry.tool_temp, 210.0);
server.stop();
```

---

## 5. Using PrintProof3D From KimCad

If you are developing **KimCad** or similar software, use the following integration flow:

1. **KimCad calls the CLI health check during setup/CI**:
   Run `python devtools/agent_health_check.py` to ensure the harness environment is healthy before starting CAD verification routines.
2. **KimCad invokes CLI validation for STL and G-code**:
   When exporting/slicing in KimCad, run `printproof3d validate-model` or `printproof3d validate-gcode` to perform static printability and profile-limit validations.
3. **KimCad consumes structured JSON/exit status**:
   Parse stdout for JSON reports or use files exported via `--output <path>`. An exit code of `0` signals validation success (or advisory warnings under `check-compatibility`); a non-zero exit code signals validation failures or warnings (for standard validation commands).
4. **KimCad may use SDK/mock adapters for simulated printer workflow tests**:
   Verify job dispatching, pause/resume, and telemetry status changes by wrapping tests with the twin simulators (e.g. `PrusaLinkMockServer`).
5. **KimCad must not claim real hardware validation**:
   Display clear disclaimers that physical printer trials must be checked manually.

---

## 6. Machine-Readable Outputs & Failure Modes

### JSON Report Structure
Validation commands output a report structured as follows:

```json
{
  "status": "pass",
  "target_printer_profile": "Prusa_MK4",
  "target_material_profile": "PLA",
  "model": {
    "file_name": "tetrahedron.stl",
    "units": "mm",
    "bounding_box": {
      "min_x": -5.0,
      "min_y": -5.0,
      "min_z": 0.0,
      "max_x": 5.0,
      "max_y": 5.0,
      "max_z": 8.66
    }
  },
  "issues": [],
  "confidence_level": "high"
}
```

### Exit Codes
- `0`: Success. Specifically:
  - For `validate-model`, `validate-gcode`, and `preflight`, a status of `pass` exits with `0`.
  - For `check-compatibility`, a status of `pass` or `warning` (advisory warning) exits with `0`.
  - For `validate-printer-profile` and `validate-material-profile`, a status of `valid` exits with `0`.
  - For `validate-profile-directory`, if all profiles in the directory are valid, it exits with `0`.
  - For `generate-printer-profile` and `generate-material-profile`, successful template generation exits with `0`.
- `1`: Failure or warning error. Specifically:
  - For `validate-model`, `validate-gcode`, and `preflight`, a status of `warning` or `fail` exits with `1`.
  - For `check-compatibility`, a status of `fail` exits with `1`.
  - For `validate-printer-profile` and `validate-material-profile`, a status of `invalid` exits with `1`.
  - For `validate-profile-directory`, if any profile in the directory is invalid, it exits with `1`.
  - Any parse, file reading, command-line parameter, or write errors exit with `1`.

### How to Surface Errors back to KimCad
If `issues` contains alerts, parse the list:
- `id`: Unique error ID.
- `severity`: `blocker`, `critical`, `major`, `minor`, `nit`.
- `message`: Detailed description of the safety violation.
- `suggested_fixes`: Array of corrective steps (e.g., "Reduce printing speed", "Reduce bed temperature").

### Common Failure Modes
- **Auth Failure (REST API)**: Returns `401 Unauthorized` if `PRINTPROOF3D_API_TOKEN` is incorrect or missing.
- **Port Collision**: Mock servers automatically bind to port `0` to prevent port collisions. If using a physical adapter config without dynamic ports, verify that the target port is free.

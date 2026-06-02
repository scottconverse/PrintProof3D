# PrintProof3D: Printability Analysis & Printer Management Engine

> [!IMPORTANT]
> **Project Status**: PrintProof3D is currently in pre-release (pre-1.0) developer engine status. Printer protocol adapters and path validation are tested against mock/simulator conformance harnesses and are not yet hardware-validated on physical machines. Build from source to compile.

PrintProof3D is a highly modular, type-safe **3D Printer Compatibility, Printability, and Integration Engine** written in Rust. 

The project provides compiler-safe data models, automated JSON Schema generation, static geometric and path printability validation for 3D meshes (STL) and pre-sliced machine files (G-code), remote printer protocol adapters, and a dynamic WebAssembly-sandboxed validation plugin system.

---

## Key Features

* **Type-Safe Domain Profiles**: Define printer hardware boundaries and material chemical properties using validated JSON data models.
* **Rigorous Geometry Audits**: Check STL meshes for manifold/watertightness issues, build volume limit violations, steep overhang slopes, and low bed-plate contact footprint risks.
* **Stateful G-Code Validation**: Accumulate toolhead coordinates statefully through motion coordinates (`G0`–`G3`) and homing commands (`G28`) to audit travel bounds and check thermal instructions against physical machine limits.
* **Sandboxed WASM Plugin Runtime**: Write custom validation policies in Rust, compile them to WebAssembly, and execute them in a restricted memory sandbox utilizing `wasmi`.
* **Standardized Printer Protocol Adapters**: Wrap printer connection controls under an asynchronous `PrinterAdapter` trait. Concrete adapter clients are implemented and verified against simulator twin mocks. No physical-printer validation or hardware certification is claimed.
* **Developer SDK**: Run mock servers and automated conformance test suites to verify custom adapter compliance.
* **Axum REST microservice & MCP Server**: Integrate validation hooks into web servers, slicers, asset databases, or AI agentic workflows.

---

## Project Structure & Crate Layout

PrintProof3D is organized as a Cargo workspace with decoupled crates:

* **[`crates/core`](crates/core)**: Contains domain structures (`PrinterProfile`, `MaterialProfile`, `ValidationReport`) and validation invariants.
* **[`crates/printability`](crates/printability)**: Mathematical geometry validation and G-code position/temperature checking.
* **[`crates/adapters`](crates/adapters)**: Standardized printer connection protocols and telemetry definitions.
* **[`crates/sdk`](crates/sdk)**: Mock connection servers and conformance test harnesses.
* **[`crates/plugins`](crates/plugins)**: WebAssembly guest loading, host/guest memory exchange, fuel metering, and linear-memory limit enforcement.
* **[`crates/cli`](crates/cli)**: Command line utility and Model Context Protocol (MCP) server.
* **[`crates/rest`](crates/rest)**: Local-loopback Axum HTTP REST server protected by Bearer Token authorization.
* **[`crates/example-plugin`](crates/example-plugin)**: Sample validation plugin compiling to `wasm32-unknown-unknown` to append volume warnings.

---

## 🚀 10-Minute Developer Quickstart

Get up and running with PrintProof3D in 10 minutes or less.

### 1. Prerequisites & Compilation
Ensure you have the Rust toolchain installed. Since PrintProof3D utilizes a WebAssembly runtime for sandboxed plugins, add the WASM build target:
```bash
rustup target add wasm32-unknown-unknown
```

Build the workspace locally:
```bash
# Compile the entire workspace in release mode
cargo build --release
```
The compiled binaries are generated in:
- **CLI Utility**: `target/release/printproof3d` (or `target/release/printproof3d.exe` on Windows)
- **REST Daemon**: `target/release/printproof3d-rest` (or `target/release/printproof3d-rest.exe` on Windows)

*Note: For global system CLI usage, you can run `cargo install --path crates/cli` to make `printproof3d` available in your path, but we recommend local testing/build paths first.*

### 2. Verify Workspace Health
Run the comprehensive agent health check script from the project root to ensure everything is set up correctly:
```bash
python devtools/agent_health_check.py
```
This runs workspace formatting checks, clippy lints, the unit/integration test suite, builds the binary, and runs validation smoke tests.

### 3. Run STL Mesh Geometry Audits
Validate a raw 3D mesh asset against target printer capabilities and material properties. The CLI validates coordinates, watertightness (manifold edges), circular bed distance thresholds, steep overhangs, and bed adhesion contact footprint area.

**Using Repository-Local Release Binary:**
```bash
# Windows
target\release\printproof3d.exe validate-model --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json

# Unix
./target/release/printproof3d validate-model --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json
```

**Using Globally Installed CLI:**
```bash
printproof3d validate-model --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json
```

### 4. Run Sliced G-Code Audits
Validate stateful travel motions, nozzle temperatures, and bed thermal limits.

**Using Repository-Local Release Binary:**
```bash
# Windows
target\release\printproof3d.exe validate-gcode --gcode fixtures/safe_print.gcode --printer profiles/prusa_mk4.json --material profiles/pla.json

# Unix
./target/release/printproof3d validate-gcode --gcode fixtures/safe_print.gcode --printer profiles/prusa_mk4.json --material profiles/pla.json
```

**Using Globally Installed CLI:**
```bash
printproof3d validate-gcode --gcode fixtures/safe_print.gcode --printer profiles/prusa_mk4.json --material profiles/pla.json
```

### 5. Unified Print Job Preflight Validation
Perform a single coherent print job preflight validation for STL geometry or sliced G-code, optionally testing connection adapter telemetry against simulator twins.

**Example 1: STL Preflight Validation**
```bash
target\release\printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json
```

**Example 2: G-code Preflight Validation**
```bash
target\release\printproof3d.exe preflight --gcode fixtures/safe_print.gcode --printer profiles/prusa_mk4.json
```

**Example 3: Simulator-Twin Preflight Connectivity Check (matching protocol)**
```bash
target\release\printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json --simulator prusalink
```

### 6. Profile Management & Compatibility CLI Commands

PrintProof3D provides subcommands for managing JSON profiles and auditing compatibility:

**Discover Profiles:**
```bash
# List printers in default profiles/ directory (JSON or text)
target\release\printproof3d.exe list-printers --format json
target\release\printproof3d.exe list-printers --directory profiles/ --format text

# List materials (JSON or text)
target\release\printproof3d.exe list-materials --format json
target\release\printproof3d.exe list-materials --directory profiles/ --format text
```

**Inspect Profiles:**
```bash
# Auto-detect profile type and display all details
target\release\printproof3d.exe inspect-profile profiles/prusa_mk4.json
target\release\printproof3d.exe inspect-profile profiles/pla.json --format json
```

**Validate Profiles:**
```bash
# Validate printer profile safety constraints
target\release\printproof3d.exe validate-printer-profile profiles/prusa_mk4.json

# Validate material profile safety constraints
target\release\printproof3d.exe validate-material-profile profiles/pla.json
```

**Auditing Multi-Dimensional Compatibility:**
```bash
# Audit printer + material profile compatibility
target\release\printproof3d.exe check-compatibility --printer profiles/prusa_mk4.json --material profiles/pla.json

# Audit printer + model compatibility
target\release\printproof3d.exe check-compatibility --printer profiles/prusa_mk4.json --model fixtures/tetrahedron.stl

# Audit printer + G-code compatibility
target\release\printproof3d.exe check-compatibility --printer profiles/prusa_mk4.json --gcode fixtures/safe_print.gcode
```

### 7. Integration Channels

- **WASM Plugins**: Compile guest plugins to the WASM target and run validations using `--plugin <path_to_wasm>`.
- **Axum REST API**: Spin up the local HTTP daemon using `cargo run --package printproof3d-rest` (listening on port `3000`, protected by Bearer token authentication).
- **AI Agentic Workflows (MCP)**: Run the Model Context Protocol JSON-RPC server over stdout/stdin using `printproof3d mcp`.
- **Crate Dependencies**: `printproof3d-core` and `printproof3d-adapters` are usable as path/git dependencies in your external Rust application's `Cargo.toml` unless/until published to crates.io.

---

## 📋 Validation Report JSON Contract

Commands output a unified machine-readable JSON report. External applications should parse the following contract structure:

```json
{
  "status": "pass",
  "target_printer_profile": "Prusa_MK4",
  "target_material_profile": "Polylactic Acid",
  "model": {
    "file_name": "tetrahedron.stl",
    "units": "mm",
    "bounding_box": {
      "min_x": 0.0,
      "min_y": 0.0,
      "min_z": 0.0,
      "max_x": 10.0,
      "max_y": 8.66,
      "max_z": 8.16
    }
  },
  "issues": [
    {
      "id": "MESH_NOT_MANIFOLD",
      "severity": "critical",
      "message": "Model mesh is not watertight/manifold. Found 3 open/non-manifold edges.",
      "location": {
        "region": "mesh_boundaries",
        "geometry": null
      },
      "suggested_fixes": [
        "Repair the 3D model in a mesh editor (e.g. Blender, Netfabb) to make it watertight."
      ]
    }
  ],
  "confidence_level": "high",
  "sliced_settings_assumed": null
}
```

### Properties
- `status`: String enum. Value can be `"pass"` (passes PrintProof3D profile and file validation checks), `"warning"` (non-blocking safety suggestions), or `"fail"` (critical hardware/extrusion issues detected).
- `issues`: Array of validation issues.
  - `id`: Unique upper-case machine identifier (e.g. `MESH_NOT_MANIFOLD`, `GCODE_OUT_OF_BOUNDS`, `HOTEND_TEMP_EXCEEDS_MAX`).
  - `severity`: String enum (`info`, `minor`, `major`, `critical`, `blocker`).
  - `message`: User-facing description.
  - `suggested_fixes`: Actionable suggestions for slicers or CAD correction.

---

## 🚦 Exit Codes
PrintProof3D returns standard shell exit codes for automated tooling integration (e.g., CI/CD pipelines or IDE task runners):
- `0`: Validation/compatibility checks pass. Specifically:
  - For `validate-model`, `validate-gcode`, and `preflight`, a status of `pass` exits with `0`.
  - For `check-compatibility`, a status of `pass` or `warning` (advisory warning) exits with `0`.
- `1`: Validation/compatibility checks fail, warnings are treated as errors (where applicable), or a system error occurs. Specifically:
  - For `validate-model`, `validate-gcode`, and `preflight`, a status of `warning` or `fail` exits with `1`.
  - For `check-compatibility`, a status of `fail` exits with `1`.
  - Any parse, file reading, or command-line usage errors exit with `1`.

---

## ⚠️ Simulator-Only SDK & Adapter Limitations
> [!WARNING]
> PrintProof3D is **not hardware-validated**.
> All physical communication protocols (Bambu MQTT/FTP, Moonraker/Klipper, OctoPrint, PrusaLink, RepRapFirmware, Marlin Serial) are verified solely using local, sandboxed twin simulator mocks.
> Passing validation checks indicates protocol client compliance, but does **not** certify physical printer thermal safety, real nozzle movement limits, or safety from hardware failures. Physical printing safety remains the sole responsibility of the operator.

---

## 🔧 Troubleshooting Guide

#### 1. Port Collisions in Mock Servers
- **Symptom:** SDK tests fail to bind or report "address already in use."
- **Solution:** PrintProof3D mocks default to binding to dynamic ephemeral port `0` (`127.0.0.1:0`), letting the operating system select an available port. If configuring custom connection profiles, make sure to use port `0` or ensure target ports are fully free.

#### 2. Auth Failures (`401 Unauthorized`) in REST API
- **Symptom:** REST endpoint requests fail with authorization errors.
- **Solution:** Ensure you pass the header `Authorization: Bearer <token>`. The token is read from the `PRINTPROOF3D_API_TOKEN` environment variable. If `PRINTPROOF3D_API_TOKEN` is not configured, the REST daemon generates a secure ephemeral startup token and prints it to the console (e.g., `[PrintProof3D API] Token is not configured. Ephemeral token generated: <token>`). Copy this token to authenticate.

#### 3. WASM Sandboxed Plugin compilation target
- **Symptom:** Plugins fail to load or report architecture mismatch errors.
- **Solution:** Sandboxed plugins must be compiled for target `wasm32-unknown-unknown` (e.g., `cargo build --target wasm32-unknown-unknown --release`). Verify that your Rust toolchain has the target added.

#### 4. Isolated adapters/SDK compile errors
- **Symptom:** Compiling `printproof3d-adapters` or `printproof3d-sdk` in isolation fails with errors like `could not find fs in tokio`.
- **Solution:** Verify that `crates/adapters/Cargo.toml` has the `fs` feature enabled on the `tokio` dependency (i.e. `tokio = { features = ["sync", "time", "fs"], ... }`). This resolves feature union leakage requirements.

---

## Running Workspace Tests

Run all unit, integration, and conformance tests across the workspace crates:
```bash
cargo test --workspace
```

---

## Documentation Links

For details on integration, mechanics, and APIs, see the dedicated documentation files:
* **[PrintProof3D User Manual](USER_MANUAL.md)**: Deep dive on profile schemas, mathematical formulas, integration hooks, and the custom plugin tutorial.
* **[System Architecture Spec](ARCHITECTURE.md)**: Details system boundaries, WASM memory mapping sequence diagrams, and adapter state-machine flowcharts.
* **[Core API Reference Manual](API_REFERENCE.md)**: Complete guide on structures, trait functions, error enums, and exports macros.

# PrintProof3D Release Checklist & Guide (v0.5.0-rc1)

This checklist and guide outlines the packaging, platform compatibility, and verification steps necessary to declare a Release Candidate for **PrintProof3D**.

---

## 1. Supported Platforms & Requirements

### Supported & Verified Platforms
The following platforms are fully verified and tested via local smoke tests and GitHub Actions CI runs for this Release Candidate:
* **Windows**: Windows 10 / 11 (verified locally on Windows PowerShell & cmd.exe).
* **Linux**: Ubuntu 22.04+ (verified on GitHub Actions CI; requires `libudev-dev` for serial connection adapter compiling).

### Intended Platform Compatibility (Not Verified for this RC)
* **macOS**: macOS 12+ (Apple Silicon & Intel). macOS-specific compilation and runtime behaviors are designed for cross-platform compatibility but have not been natively verified for this Release Candidate due to environment availability.

### Build & Run Prerequisites
* **Rust Toolchain**: `stable` (v1.75+ or newer recommended).
* **Cargo**: Included with the Rust toolchain.
* **Python**: Python 3.8+ (required for executing development watchdog scripts and health checks).
* **Git**: System git installation for revision-tagging and GHA workflows.

---

## 2. Simulator-Only Limitations (Disclaimer)

> [!WARNING]
> **Hard Limit: Simulator-Verification Only**
> PrintProof3D is a software-limits static checker and interface conformance test harness.
> - A "pass" status indicates only that the sliced files or printer profiles pass PrintProof3D's static rules and schema validations.
> - **It does NOT certify physical printer operation, prevent heater or motion faults, prevent mechanical collisions, or guarantee completed prints on physical 3D printers.**
> - All remote printer protocol adapters (Bambu Lab MQTT/FTP, Moonraker/Klipper, OctoPrint, PrusaLink, RepRapFirmware, Marlin Serial) are validated against simulated twin mocks only. 
> - Users must manually verify physical safety rules and follow printer manufacturer safety instructions before starting prints.

---

## 3. Build & Compilation Instructions

### Compiling the CLI Binary
To compile the optimized release binary of the command-line utility, run:
```powershell
cargo build --release --bin printproof3d
```
The compiled executable will be written to:
* **Windows**: `target/release/printproof3d.exe`
* **Unix**: `target/release/printproof3d`

### Compiling the REST Service
To compile the release binary of the Axum web service:
```powershell
cargo build --release --bin printproof3d-rest
```
The compiled executable will be written to:
* **Windows**: `target/release/printproof3d-rest.exe`
* **Unix**: `target/release/printproof3d-rest`

---

## 4. Running Guidelines

### Running the CLI
Run validations directly using the release binary:
```powershell
target/release/printproof3d.exe validate-model --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json
```

### Running the REST Daemon
To run the server locally on port `3000`:
```powershell
target/release/printproof3d-rest.exe
```
Ensure you provide the authorization header in API requests:
`Authorization: Bearer secret_print_token`

### Running the MCP JSON-RPC Server
Expose the validation capability to AI agents:
```powershell
target/release/printproof3d.exe mcp
```

### Running the Agent Health Check
To verify overall repository integrity and local runner health:
```powershell
python devtools/agent_health_check.py
```

---

## 5. Pre-Release Verification Gates

Before tagging or shipping any release candidate, the following steps must be completed:
1. [ ] **Fmt**: `cargo fmt --all -- --check` completes successfully.
2. [ ] **Clippy**: `cargo clippy --workspace --all-targets -- -D warnings` returns zero lints/warnings.
3. [ ] **Tests**: `cargo test --workspace` passes all tests.
4. [ ] **Builds**: Release builds for CLI and REST compile without errors.
5. [ ] **Health**: `python devtools/agent_health_check.py` returns `Health check PASSED`.
6. [ ] **Forbidden Language Scan**: Verify zero overclaiming safety phrases are present.

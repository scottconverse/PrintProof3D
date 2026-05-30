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
* **Standardized Printer Protocol Adapters**: Wrap printer connection controls under an asynchronous `PrinterAdapter` trait (trait and mock conformance harness implemented; Moonraker/OctoPrint concrete clients are currently trait stubs).
* **Developer SDK**: Run mock servers and automated conformance test suites to verify custom adapter compliance.
* **Axum REST microservice & MCP Server**: Integrate validation hooks into web servers, slicers, asset databases, or AI agentic workflows.

---

## Project Structure & Crate Layout

PrintProof3D is organized as a Cargo workspace with decoupled crates:

* **[`crates/core`](crates/core)**: Contains domain structures (`PrinterProfile`, `MaterialProfile`, `ValidationReport`) and validation invariants.
* **[`crates/printability`](crates/printability)**: Mathematical geometry validation and G-code position/temperature checking.
* **[`crates/adapters`](crates/adapters)**: Standardized printer connection protocols and telemetry definitions.
* **[`crates/sdk`](crates/sdk)**: Mock connection servers and conformance test harnesses.
* **[`crates/plugins`](crates/plugins)**: WebAssembly guest loading and linear memory management stubs.
* **[`crates/cli`](crates/cli)**: Command line utility and Model Context Protocol (MCP) server.
* **[`crates/rest`](crates/rest)**: Local-loopback Axum HTTP REST server protected by Bearer Token authorization.
* **[`crates/example-plugin`](crates/example-plugin)**: Sample validation plugin compiling to `wasm32-unknown-unknown` to append volume warnings.

---

## ⚙️ Installation & Setup

PrintProof3D must be compiled from source. Follow these steps to build the workspace locally:

### 1. Build from Source (Recommended Local Path)
```bash
# Clone the repository
git clone https://github.com/scottconverse/PrintProof3D.git
cd PrintProof3D

# Compile the entire workspace in release mode
cargo build --release
```
The compiled binaries are written to the target subdirectory:
* **CLI tool**: `./target/release/printproof3d` (or `./target/release/printproof3d.exe` on Windows)
* **REST server**: `./target/release/printproof3d-rest` (or `./target/release/printproof3d-rest.exe` on Windows)

### 2. Global Installation
If you want to invoke `printproof3d` globally in your path, compile and install via cargo path:
```bash
# Install the command-line tool globally
cargo install --path crates/cli

# Verify CLI installation
printproof3d --version

# Install the REST API server globally
cargo install --path crates/rest

# Verify REST server installation
printproof3d-rest --help
```
> [!NOTE]
> Ensure that Cargo's binary installation path (typically `~/.cargo/bin` on Unix systems, or `%USERPROFILE%\.cargo\bin` on Windows) is present in your system's `PATH` environment variable.

---

## Quickstart & Commands

### 1. Compile the Example Plugin
Build all native crates, unit tests, and compile the example WebAssembly validation plugin:
```bash
# Build the native project crates
cargo build --release

# Compile the sample WASM rules plugin
cargo build --package example-plugin --target wasm32-unknown-unknown --release
```

### 2. Execute Geometry Validation
Audit raw 3D mesh assets against target hardware and material limits:
```bash
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

### 3. Execute Toolpath Validation
Audit sliced G-code instructions for safety bounds:
```bash
printproof3d validate-gcode \
  --gcode fixtures/safe_print.gcode \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

### 4. Run Validation with Custom WASM Plugins
Inject custom compliance policies at runtime using the compiled WASM binary:
```bash
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json \
  --plugin target/wasm32-unknown-unknown/release/example_plugin.wasm
```

### 5. Spin up the REST Web Daemon
Launch the HTTP validation microservice:
```bash
cargo run --package printproof3d-rest
```
*Secure routes enforce Bearer authentication. By default, the API key defaults to `secret_print_token`.*

### 6. Interface with AI Agents (MCP Server)
Integrate validation rules into agentic software assistants (like Cursor or Claude Desktop):
```bash
printproof3d mcp
```

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

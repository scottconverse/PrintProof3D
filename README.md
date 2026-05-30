# PrintProof3D: Printability Analysis & Printer Management Engine

PrintProof3D is a highly modular, type-safe **3D Printer Compatibility, Printability, and Integration Engine** written in Rust. 

The project provides compiler-safe data models, automated JSON Schema generation, static geometric and path printability validation for 3D meshes (STL) and pre-sliced machine files (G-code), remote printer protocol adapters, and a dynamic WebAssembly-sandboxed validation plugin system.

---

## Key Features

* **Type-Safe Domain Profiles**: Define printer hardware boundaries and material chemical properties using structured, validated JSON data models.
* **Rigorous Geometry Audits**: Check STL meshes for manifold/watertightness issues, build volume limit violations, steep overhang slopes, and low bed-plate contact footprint risks.
* **Stateful G-Code Validation**: Accumulate toolhead coordinates statefully through motion coordinates (`G0`–`G3`) and homing commands (`G28`) to audit travel bounds and check thermal instructions against physical machine limits.
* **Sandboxed WASM Plugin Runtime**: Write custom enterprise safety and compliance policies in Rust, compile them to WebAssembly, and execute them in a restricted memory sandbox utilizing `wasmi`.
* **Standardized Printer Protocol Adapters**: Wrap Moonraker/Klipper, OctoPrint, and Duet/RRF connection controls under an asynchronous `PrinterAdapter` trait.
* **Developer SDK**: Run mock servers and automated conformance test suites to verify custom adapter compliance.
* **Axum REST microservice & MCP Server**: Integrate validation hooks into web servers, slicers, asset databases, or AI agentic workflows.

---

## Project Structure & Crate Layout

PrintProof3D is organized as a Cargo workspace with decoupled crates:

* **[`crates/core`](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/crates/core)**: Contains domain structures (`PrinterProfile`, `MaterialProfile`, `ValidationReport`) and validation invariants.
* **[`crates/printability`](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/crates/printability)**: Mathematical geometry validation and G-code position/temperature checking.
* **[`crates/adapters`](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/crates/adapters)**: Standardized printer connection protocols and telemetry definitions.
* **[`crates/sdk`](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/crates/sdk)**: Mock connection servers and conformance test harnesses.
* **[`crates/plugins`](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/crates/plugins)**: WebAssembly guest loading and linear memory management stubs.
* **[`crates/cli`](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/crates/cli)**: Command line utility and Model Context Protocol (MCP) server.
* **[`crates/rest`](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/crates/rest)**: Local-loopback Axum HTTP REST server protected by Bearer Token authorization.
* **[`crates/example-plugin`](file:///C:/Users/scott/Documents/antigravity/eager-archimedes/PrintProof3D/crates/example-plugin)**: Sample validation plugin compiling to `wasm32-unknown-unknown` to append volume warnings.

---

## Quickstart & Commands

### 1. Compile the Workspace
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

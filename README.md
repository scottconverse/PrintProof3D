# PrintProof3D

PrintProof3D is a highly modular, type-safe **3D Printer Compatibility, Printability, and Integration Engine** written in Rust. It provides compiler-safe data models, automated JSON Schema generation, static geometric and path printability verification for STL/G-code, printer protocol adapters, and a dynamic WebAssembly-sandboxed plugin system.

---

## 🛠 Crate Layout & Crate Hierarchy

The project is structured as a Cargo workspace split into decoupled crates:

* **[crates/core](file:///crates/core)**: Versioned JSON schemas and core configurations (`PrinterProfile`, `MaterialProfile`, `ValidationReport`).
* **[crates/printability](file:///crates/printability)**: Geometry validation (STL watertightness, overhang bounds) and G-code position/thermal safety checkers.
* **[crates/adapters](file:///crates/adapters)**: Connection interfaces for printer protocols (OctoPrint, Moonraker/Klipper, Marlin Serial) and mock telemetry handlers.
* **[crates/sdk](file:///crates/sdk)**: Integration SDK exposing automated adapter conformance verification suites.
* **[crates/plugins](file:///crates/plugins)**: WebAssembly sandbox loader utilizing the pure-Rust `wasmi` interpreter for safe, dynamic plugin execution.
* **[crates/cli](file:///crates/cli)**: Command line utility and Model Context Protocol (MCP) server integration.
* **[crates/rest](file:///crates/rest)**: Local-loopback Axum HTTP REST server protected by Bearer Token Auth.
* **[crates/example-plugin](file:///crates/example-plugin)**: Sample validation plugin compiling to `wasm32-unknown-unknown` to append volume warnings.

---

## 🚀 Quickstart & Usage

### 1. Model and G-code Validation via CLI

Inspect a raw 3D mesh (STL) or pre-sliced G-code using CLI commands:

```bash
# Validate STL mesh geometry compatibility
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json

# Validate G-code coordinates and thermal safety limits
printproof3d validate-gcode \
  --gcode fixtures/safe_print.gcode \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

### 2. Loading Dynamic WASM Validation Plugins

Enrich reports with dynamic custom rules loaded securely at runtime:

```bash
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json \
  --plugin target/wasm32-unknown-unknown/debug/example_plugin.wasm
```

### 3. Running the REST API Server

Launch the Axum REST API server bound to local port `3000`:

```bash
cargo run --package printproof3d-rest
```
Verify endpoints like `/profiles/printers` or perform multipart validation requests `/validate/model` using a Bearer token.

### 4. Running the MCP JSON-RPC Server

Integrate PrintProof3D validation tools into AI agentic workflows (e.g. Cursor, MCP Clients):

```bash
printproof3d mcp
```

---

## 🧪 Developer SDK & Compliance Harness

Developer integrations can import print adapters and verify compliance using the conformance suite:

```rust
use printproof3d_adapters::PrinterAdapter;
use printproof3d_sdk::run_conformance_tests;

#[tokio::test]
async fn verify_custom_adapter() {
    let mut my_adapter = MyPrinterClient::new();
    let result = run_conformance_tests(&mut my_adapter).await;
    assert!(result.is_ok(), "Adapter failed compliance tests: {:?}", result);
}
```

---

## 🔒 Security & Sandbox Guarantees

* **Zero System Access**: WASM Plugins run inside a hermetic `wasmi` environment with no raw access to sockets, filesystem, or operating system calls.
* **Type-Safe Serializations**: Data exchange occurs strictly via JSON strings copied over shared WASM memory buffers (`alloc`/`dealloc`/`validate`).
* **Auth Middleware**: REST endpoints enforce Bearer Token verification on every route.

---

## 🔨 Building & Testing

To build the workspace and run all tests (pre-push hooks automatically verify this):

```bash
# Run all workspace unit, integration, and WASM tests
cargo test --workspace

# Compile the example validation plugin to WebAssembly
cargo build --package example-plugin --target wasm32-unknown-unknown
```

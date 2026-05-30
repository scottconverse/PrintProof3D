# PrintProof3D

PrintProof3D is a highly modular 3D Printer Compatibility, Printability, and Integration Engine written in Rust. It provides compile-time safe data models, automated JSON schema generation, geometric printability verification for STL/G-code, and connection adapters for popular printer protocols.

## Key Features

- **Profile Validation**: Automated schema validation for Printer and Material profiles.
- **Printability Inspection**: Automated geometric checks for overhangs, manifold boundaries, and build volume bounds.
- **Protocol Adapters**: Integrated support for Moonraker (Klipper), OctoPrint, and Marlin Serial.
- **Developer SDK**: Comprehensive SDK for custom integration and conformance validation.
- **CLI & MCP Server**: Command Line Interface for direct integration and a Model Context Protocol (MCP) server for LLM tools.

---

## Workspace Structure

The project is structured as a Cargo workspace with five distinct crates:

* **[crates/core](file:///crates/core)**: Core data models (`PrinterProfile`, `MaterialProfile`, `ValidationReport`) and schema auto-generation.
* **[crates/printability](file:///crates/printability)**: Mesh validation algorithms and G-code boundary checkers.
* **[crates/adapters](file:///crates/adapters)**: Connection adapters for printer frontends (OctoPrint, Moonraker) and serial protocols.
* **[crates/sdk](file:///crates/sdk)**: Integration SDK and adapter conformance test harnesses.
* **[crates/cli](file:///crates/cli)**: Command line utility and Model Context Protocol (MCP) server.

---

## Installation

### As a CLI Tool

Build and install the CLI binary locally:

```bash
cargo install --path crates/cli
```

### As a Library Dependency

Add the core and SDK crates to your `Cargo.toml`:

```toml
[dependencies]
printproof3d-core = { path = "crates/core" }
printproof3d-sdk = { path = "crates/sdk" }
```

---

## Quickstart

### CLI Usage

```bash
# Verify printability of an STL file
printproof3d validate --model fixtures/tetrahedron.stl --printer profiles/mk4.json --material profiles/pla.json

# Launch the Model Context Protocol (MCP) server
printproof3d mcp
```

### Library Usage

```rust
use printproof3d_core::{PrinterProfile, MaterialProfile, BuildVolume};

fn main() {
    let build_vol = BuildVolume { x: 250.0, y: 210.0, z: 220.0 };
    println!("Build volume loaded: {}x{}x{}", build_vol.x, build_vol.y, build_vol.z);
}
```

---

## Building & Testing

To build the workspace and execute all unit and integration tests:

```bash
cargo test --workspace
```

> [!NOTE]
> Running tests will automatically output the updated compiled JSON schemas to the `/schemas` directory at the workspace root.

---

## License

This project is licensed under the MIT License. See the parent repository's [LICENSE](file:///../LICENSE) for details.

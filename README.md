# PrintProof3D

PrintProof3D is a highly modular 3D Printer Compatibility, Printability, and Integration Engine written in Rust. It provides compile-time safe data models, automated JSON schema generation, geometric printability verification for STL/G-code, and connection adapters for popular printer protocols.

## Key Features

- **Profile Validation**: Automated schema validation for Printer and Material profiles.
- **Printability Inspection**: Automated geometric checks for overhangs, manifold boundaries, and build volume bounds.
- **Protocol Adapters**: Integrated support for Moonraker (Klipper), OctoPrint, and Marlin Serial.
- **Developer SDK**: Comprehensive SDK for custom integration and conformance validation.
- **CLI & MCP Server**: Command Line Interface for direct integration (MCP Server integration planned for Stage 2).

---

## Workspace Structure

The project is structured as a Cargo workspace with five distinct crates:

* **[crates/core](file:///crates/core)**: Core data models (`PrinterProfile`, `MaterialProfile`, `ValidationReport`) and schema auto-generation.
* **[crates/printability](file:///crates/printability)**: Mesh validation algorithms and G-code boundary checkers.
* **[crates/adapters](file:///crates/adapters)**: Connection adapters for printer frontends (OctoPrint, Moonraker) and serial protocols.
* **[crates/sdk](file:///crates/sdk)**: Integration SDK and adapter conformance test harnesses.
* **[crates/cli](file:///crates/cli)**: Command line utility.

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

Ensure profiles are located in the appropriate paths or provide direct JSON file paths:

```bash
# Verify compatibility of an STL file against printer and material profiles
printproof3d validate-model --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json

# Verify compatibility of a G-code file against a printer profile
printproof3d validate-gcode --gcode fixtures/cube_safe.gcode --printer profiles/prusa_mk4.json
```

### Library Usage

```rust
use printproof3d_core::{BuildVolume};

fn main() {
    let build_vol = BuildVolume::Rectangular { x: 250.0, y: 210.0, z: 220.0 };
    match build_vol {
        BuildVolume::Rectangular { x, y, z } => {
            println!("Rectangular Build volume loaded: {}x{}x{} mm", x, y, z);
        }
        BuildVolume::Cylindrical { diameter, z } => {
            println!("Cylindrical Build volume loaded: diameter {} mm, height {} mm", diameter, z);
        }
    }
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

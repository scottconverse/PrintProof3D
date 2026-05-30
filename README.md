# PrintProof3D

A 3D Printer Compatibility, Printability, and Integration Engine written in Rust.

## Crate Layout
* `crates/core`: Shared schemas for printer profiles, material profiles, and validation reports. It also auto-generates JSON schemas on test execution.
* `crates/printability`: Mesh validation algorithms and G-code static boundary checkers.
* `crates/adapters`: Core connection adapters for OctoPrint, Moonraker, and Marlin serial.
* `crates/sdk`: Developer SDK and adapter conformance test harnesses.
* `crates/cli`: Command Line Interface and MCP server.

## Building & Testing

Run tests and compile:
```bash
cargo test --workspace
```

The tests will automatically output the compiled JSON schemas to the `/schemas` directory at the workspace root.

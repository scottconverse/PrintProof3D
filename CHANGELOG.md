# Changelog

All notable changes to the PrintProof3D project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.4.0] - 2026-05-30

### Added
- **Asynchronous Network Adapters**: Introduced the `PrinterAdapter` trait in `printproof3d-adapters` for standardizing connection layers (Moonraker/Klipper, OctoPrint, Duet/RepRapFirmware).
- **Axum REST microservice**: Built a condition-compliant Axum web daemon in `printproof3d-rest` providing multipart API endpoints (`/validate/model`, `/validate/gcode`) protected by Bearer token authorization middleware.
- **Model Context Protocol (MCP)**: Implemented an MCP JSON-RPC 2.0 server over standard I/O in `printproof3d-cli` exposing printability tools (`validate_model_printability`, `validate_gcode`, `list_printer_profiles`, `explain_validation_report`) directly to AI agent assistants.
- **Harness Compliance SDK**: Created an automated compliance testing runner `run_conformance_tests` in `printproof3d-sdk` alongside network mock servers to verify adapter telemetry stability and state transitions.

---

## [0.3.0] - 2026-05-30

### Added
- **Sandboxed WebAssembly Interpreter**: Configured the `wasmi` linear memory interpreter in `printproof3d-plugins` to load dynamic validation rules at runtime without local filesystem access.
- **Shared Memory Protocol**: Designed a pointer-length buffer exchange protocol (`alloc`/`dealloc`/`validate`) utilizing a packed 64-bit return layout to copy report JSON strings over WASM boundaries.
- **Rust Export Macro**: Created the `export_validation_plugin!` macro to automatically generate guest memory wrappers.
- **Example Plugin Crate**: Created `crates/example-plugin` to check minimum volume constraints.

---

## [0.2.0] - 2026-05-30

### Added
- **STL Geometry Checkers**: Implemented manifold boundary checking (using $10^{-3}$ float-to-integer key quantization), spatial build volume fitting, overhang tilt slope cosine calculations, and contact area bed adhesion ratio calculations.
- **Stateful G-Code Parser**: Built a stateful coordinates accumulator tracking homing (`G28`) and travel bounds changes (`G0`–`G3`) under absolute (`G90`) and relative (`G91`) modes.
- **G-Code Thermal Inspector**: Added parsing checks for heater setup instructions (`M104`, `M109`, `M140`, `M190`) comparing targets against profile limits.

---

## [0.1.0] - 2026-05-30

### Added
- **Core Workspace Layout**: Scaffolded the cargo project structure containing crates for `core`, `printability`, `adapters`, `sdk`, and `cli`.
- **JSON Profile Schemas**: Defined the type-safe models for `PrinterProfile`, `MaterialProfile`, and `ValidationReport` with automated `schemars` serialization unit tests.
- **Asset Fixtures Library**: Added STL meshes and pre-sliced G-code travel assets under the `/fixtures` folder.
- **Git Hook Setup**: Configured pre-push hooks to enforce workspace formatting, clean clippy builds, and passing test suites.

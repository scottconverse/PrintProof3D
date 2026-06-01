# Changelog

All notable changes to the PrintProof3D project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0-rc1] - 2026-06-01

### Added
- **Directory Validation**: Introduced the `validate-profile-directory` command to validate all printer and material profiles in a folder, returning non-zero codes on any invalid configuration files.
- **Multidimensional Compatibility Check**: Added the `check-compatibility` command and corresponding REST endpoint `/validate/compatibility` to audit the alignment of printer limits, material limits, STL mesh geometry, and G-code tools.
- **Advanced STL Geometry Validation**: Static mesh checks for bounding box limits (`MODEL_OVERSIZED`), bed surface alignment (`BELOW_BED_GEOMETRY`), and zero-area or quantized duplicates (`DEGENERATE_TRIANGLES`).
- **Stateful G-code Path Auditing**: Static path checks for disallowed formats (`UNSUPPORTED_FILE_TYPE`), blacklisted command profiles (`UNSAFE_COMMAND_BLOCKED`), homing setup validation (`MISSING_HOMING`), and cold extrusion checks (`COLD_EXTRUSION`).
- **Profile Generation Utilities**: Commands to output standard default profile templates for printers and materials.
- **Axum API Server Completeness**: Bearer-authenticated endpoints for profile inspection (`POST /profiles/inspect`), profile validation (`POST /profiles/validate/printer`, `POST /profiles/validate/material`), and compatibility verification.

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

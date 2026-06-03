# Changelog

All notable changes to the PrintProof3D project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-06-03

### Fixed
- **Overhang/bridge/bed-contact detection no longer trusts STL file normals.** The mesh validator
  read each facet's normal straight from the STL file for its downward-facing checks and *skipped*
  any facet whose stored normal was zero-length. Many exporters (and most binary STLs) write
  `0 0 0` normals and leave the slicer to derive them from winding — so a model with zeroed/garbage
  normals silently produced **no** `OVERHANG_UNSUPPORTED`/`BRIDGE_UNSUPPORTED` findings (and a false
  `POOR_BED_ADHESION`), flipping a real `warning` to a `pass`. The checks now fall back to the
  geometric normal computed from the vertex winding when the stored normal is missing/degenerate
  (files with valid normals are unchanged). New `effective_facet_normal` helper + tests.
- **`MODEL_OUT_OF_BOUNDS` no longer false-fails a model resting on the bed.** The build-volume
  bounds check used a hard `< 0.0` lower bound with no tolerance, contradicting the dedicated
  below-bed check's `-0.05 mm` tolerance — so a model sitting on the bed (or placed flush in a
  corner) with sub-millimeter float/placement noise (e.g. `Z min = -0.03`) was reported as a
  `Critical` out-of-bounds `fail`. The rectangular and cylindrical bounds checks now apply a
  `BUILD_VOLUME_TOL` (0.05 mm, matching the below-bed check) to both lower and upper bounds; a model
  genuinely outside the volume still fails. New tests.
- **Stabilized UI/UX behaviors and local prechecks.** Restored the punch-list tracking closure matrix. Added direct browser acceptance tests for WebGL fallback mode, pending loaders, validation reset states, and token autocomplete attributes. Extended client-side volume checks to verify circular/cylindrical printer profiles with multi-vertex distance validation.

## [0.5.0-rc2] - 2026-06-02

### Added
- **REST Navigation Aliases**: Integrated route mapping and redirects for relative `.html` paths in documentation routes, avoiding 404 errors under serve environments.
- **Link Check Integration Tests**: Added automated link-crawling checks that query and assert `200 OK` on every internal nav link.
- **Hardened Validation Error Handlers**: Modified STL, G-code, and profile inspection/validation endpoints to catch and return structured `400 Bad Request` JSON responses for malformed uploads, missing multipart fields, and invalid profiles.

### Changed
- **Token Guidance Alignment**: Cleaned up all user-facing documentation to replace production static print token guidance with environment variable config (`PRINTPROOF3D_API_TOKEN`) and ephemeral console token logs.
- **API Reference Duplication Cleanup**: Removed duplicate section blocks from `API_REFERENCE.md` and aligned `api_reference.html`.
- **Policy Check Scope**: Extended docs compliance scanner limits to run documentation-scoped checks across all repository `.md` and `.html` documentation surfaces.

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

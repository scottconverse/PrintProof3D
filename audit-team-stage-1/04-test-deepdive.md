# Test Suite Deep-Dive — PrintProof3D

**Audit date:** 2026-05-30
**Role:** Senior Test Engineer
**Scope audited:** Workspace sub-crates (`core`, `printability`, `adapters`, `sdk`, `cli`), fixtures (`fixtures/` directory), schemas (`schemas/` directory), Git hooks (`.git/hooks/pre-push`), GitHub Actions (`.github/workflows/ci.yml`), and cargo dependency configurations.
**Auditor posture:** Adversarial (professionally paranoid test engineering perspective)

---

## TL;DR

The PrintProof3D test suite has evolved since the initial audit, with some critical flakiness and coverage issues resolved.
* **Resolved Issues:** 
  * Automated CI/CD workflows (`TEST-002`) are in place via GitHub Actions.
  * Boundary and validation checks on profiles (`TEST-004`) are implemented in `crates/core/src/lib.rs` along with robust unit tests.
  * Hardcoded ports and non-deterministic sleep durations in mock server tests (`TEST-006`) have been resolved by moving mocks to dynamic port assignment (port `:0`) and removing thread sleeps from tests.
  * The MQTT mock server (`BambuMqttMock`) is now fully tested (`TEST-007`) via a new unit test verifying connection, subscription, and telemetry loop.
* **Unresolved / Persistent Gaps:** 
  * The core verification engines (`printability` crate, G-code static boundary checkers) and the connection adapters (`adapters` crate) remain unimplemented skeleton placeholders. The test suite is still "mock-green" because the core functionality has zero unit or integration tests (`TEST-001`).
  * The test fixture library (`TEST-003`) containing STLs and G-code files remains completely disconnected from the test suite.
  * Schema generation still causes side-effects within a unit test (`TEST-005`).
  * There are no CLI integration or E2E tests (`TEST-008`) to ensure the command-line interface does not regress.

## Severity roll-up (tests)

Active findings:

| Severity | Count |
|---|---|
| Blocker | 0 |
| Critical | 2 |
| Major | 1 |
| Minor | 1 |
| Nit | 0 |

Resolved findings: 4

## What's working

- **Dynamic Mock Server Port Binding** — Mock servers (`RrfMockServer`, `BambuFtpMock`, `BambuMqttMock`) now dynamically bind to port `0`, eliminating port conflicts on test runners.
- **MQTT Mock Telemetry Validation** — The SDK tests now verify the full telemetry publishing behavior of `BambuMqttMock`.
- **Model Range Validation** — `crates/core/src/lib.rs` successfully enforces bounds on model fields (`PrinterProfile::validate`, `MaterialProfile::validate`, `ValidationReport::validate`) and contains unit tests verifying both correct inputs and expected errors for out-of-bounds parameters.
- **CLI Argument Parsing** — The CLI crate (`crates/cli/src/main.rs`) successfully parses subcommands (`validate-model`, `validate-gcode`), arguments, and reads file paths from the disk.
- **Automated CI Workflow** — `.github/workflows/ci.yml` successfully builds the workspace and runs tests on push/pull requests.
- **Pre-Push Gate Scripting** — The local git hook at `.git/hooks/pre-push` remains configured to run `cargo test --workspace` and block pushes if the compilation or test run fails.

## What couldn't be assessed

- **Real Printability / G-code Validation** — The actual logic to analyze STL meshes or parse/validate G-code streams remains unwritten; `check_model()` in `crates/printability/src/lib.rs` still returns static `"ok"` stubs, and the CLI generates hardcoded passing validation reports.
- **Real Connection Adapters** — The `adapters` crate defines the `PrinterAdapter` trait but contains zero implementations (e.g. Moonraker, OctoPrint, Marlin) and no tests.

---

## Test landscape

| Dimension | Observation |
|---|---|
| Framework(s) | Rust standard testing framework (`cargo test`) |
| Test pyramid shape | Sparse Unit (8 tests total: 5 in `core`, 3 in `sdk`) / No integration tests / No E2E tests |
| Coverage tool | None configured |
| Reported coverage (if any) | None (actual functional coverage of printability/adapters/CLI is 0%) |
| Flakiness posture | Significantly improved; main port-binding conflicts and sleep-based timing issues have been eliminated, though a minor flakiness risk exists in the FTP PASV data port |
| CI blocking? | Yes, GitHub Actions CI workflow blocks PRs/commits on build or test failure |

---

## Findings

> **Finding ID prefix:** `TEST-`
> **Categories:** Coverage / Shortcut / Flakiness / Quality / Ergonomics / Mocking / Regression / CI

### Active Findings

#### [TEST-001] — Critical — Quality / Coverage — Skeleton Implementation with 100% Dummy Success and Zero Feature Tests

**Evidence**
- `crates/printability/src/lib.rs` (lines 5-7):
  ```rust
  pub fn check_model() -> &'static str {
      "ok"
  }
  ```
- `crates/adapters/src/lib.rs` (lines 6-8):
  ```rust
  pub fn list_adapters() -> Vec<&'static str> {
      vec!["moonraker", "octoprint", "marlin"]
  }
  ```
- `crates/cli/src/main.rs` generates a dummy report for model and G-code validation:
  ```rust
  let report = ValidationReport {
      status: ValidationStatus::Pass,
      target_printer_profile: format!("{}_{}", printer_profile.manufacturer, printer_profile.model),
      target_material_profile: material_profile.name.clone(),
      model: ModelMetadata {
          file_name: model.file_name().unwrap_or_default().to_string_lossy().into_owned(),
          units: "mm".to_string(),
          bounding_box: BuildVolume::Rectangular { x: 50.0, y: 50.0, z: 50.0 },
      },
      issues: vec![],
      confidence_level: "high".to_string(),
      sliced_settings_assumed: None,
  };
  ```
- Zero test definitions exist inside `crates/printability` or `crates/adapters`.

**Why this matters**
The project compiles and tests pass successfully only because the core functional validation logic and integration adapters are unwritten skeletons. This masks the complete absence of application behavior and provides a false sense of security (the tests are green, but the application is empty). While basic profile range validations are now implemented in `core`, the actual verification engine remains untested skeleton code.

**Blast radius**
- Workspace crates: `printproof3d-printability`, `printproof3d-adapters`, `printproof3d-sdk`, `printproof3d` (CLI).

**Fix path**
Implement actual STL/mesh validation and G-code parsing engines. Write unit and integration tests exercising genuine geometric edge cases and syntax limits.

---

#### [TEST-003] — Major — Coverage / Quality — Test Fixture Library is Completely Disconnected and Unused

**Evidence**
- The root `fixtures/` directory contains files like `open_triangle.stl`, `overhang_flange.stl`, `tetrahedron.stl`, `out_of_bounds.gcode`, `safe_print.gcode`, and `unsafe_temp.gcode`.
- A workspace-wide search for "fixtures" yields zero occurrences in any `.rs` file.

**Why this matters**
Having a fixture library without reading or parsing files in the test suite is dead weight. If a bug is introduced in the parser or geometry processor, the test suite won't catch it because the test suite does not actually parse these fixtures.

**Blast radius**
- `printability` crate and mesh analysis paths.

**Fix path**
Add integration tests in `crates/printability` that read these fixtures from disk using relative paths (e.g. `std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/...")`), parse them, and assert that the printability engine flags the non-manifold mesh (`open_triangle.stl`), detects the overhang issue, and correctly identifies out-of-bounds/unsafe G-code blocks.

---

#### [TEST-005] — Minor — Ergonomics / CI — Schema Generation Side-Effects Inside Cargo Unit Test

**Evidence**
- In `crates/core/src/lib.rs` (lines 494-515), the `generate_schemas()` test writes JSON schema files to the local filesystem:
  ```rust
  let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
  create_dir_all(&schema_dir).unwrap();
  // ... writes printer_profile.schema.json, material_profile.schema.json, validation_report.schema.json
  ```

**Why this matters**
Unit tests should be hermetic, side-effect-free, and safe to execute concurrently. Writing files during a unit test run can lead to test failures in read-only environments (e.g., restricted CI runners) or file-locking conflicts in parallel test environments.

**Blast radius**
- `printproof3d-core` unit tests.

**Fix path**
Move the schema generation logic out of the cargo unit tests. Create a separate target bin (e.g. `crates/core/src/bin/generate_schemas.rs`) or a `build.rs` script that is run explicitly when schemas need to be regenerated, leaving `cargo test` clean and side-effect free.

---

#### [TEST-008] — Critical — Coverage / Quality — Complete Lack of CLI Integration and E2E Tests

**Evidence**
- `crates/cli/src/main.rs` contains Clap subcommands, file loading, JSON parsing, profile validations, and validation report writing logic.
- The `crates/cli` crate contains zero unit or integration tests.

**Why this matters**
The CLI is the primary entry point for users. Without automated CLI tests, regressions in command flags, file reading, exit codes, and output formatting will pass through CI undetected.

**Blast radius**
- `printproof3d` (CLI).

**Fix path**
Add integration tests in `crates/cli` (using development dependencies like `assert_cmd` and `tempfile`) to execute the compiled CLI binary, pass it valid and invalid profile paths, and assert on exit codes, stdout, and stderr.

---

### Resolved Findings

#### [TEST-002] — CI — Complete Absence of Automated CI/CD Pipelines
- **Resolution:** Fully resolved by creating `.github/workflows/ci.yml`. The workflow compiles the cargo workspace and runs tests on any push or pull request targeting the `main` or `stage-1` branches.

#### [TEST-004] — Quality — Static Happy-Path Model/Material Profile Verification without Boundary Checking
- **Resolution:** Fully resolved. Struct validation rules have been added (`PrinterProfile::validate`, `MaterialProfile::validate`, and `ValidationReport::validate` in `crates/core/src/lib.rs`). Unit tests have been added to verify that invalid inputs (negative build volume, too high nozzle/bed temperatures, invalid status invariants) are correctly detected and rejected.

#### [TEST-006] — Mocking / Flakiness — Hardcoded Ports and Non-Deterministic Sleep Durations in Mock Server Tests
- **Resolution:** Resolved. The mock servers (`RrfMockServer`, `BambuFtpMock`, `BambuMqttMock`) have been refactored to bind to port `0` (letting the OS assign a random free ephemeral port), and they expose the allocated port via a `port` field. The unit tests in `crates/sdk/src/lib.rs` now connect dynamically using `server.port`. In addition, the arbitrary `thread::sleep(Duration::from_millis(150))` calls have been removed from the test setup, eliminating test flakiness under CPU contention.
- **Residual Risk:** A minor residual risk remains in `BambuFtpMock::start()`, where the passive mode data connection (`PASV`) still attempts to bind to hardcoded port `10240` (line 43 of `crates/sdk/src/mocks/bambu.rs`). This could cause port collision errors if multiple concurrent FTP data transfers are triggered in parallel tests.

#### [TEST-007] — Coverage — Untested BambuMqttMock Implementation
- **Resolution:** Resolved. A unit test `test_bambu_mqtt_mock` has been added in `crates/sdk/src/lib.rs` (lines 44-84). This test instantiates `BambuMqttMock` dynamically, connects a `TcpStream` client, performs an MQTT handshake (Connect/Connack), subscribes to topic "test" (Subscribe/Suback), and successfully parses and asserts on a mock JSON telemetry payload (`gcode_state` is `"IDLE"`).

---

## Shortcut census

| Shortcut pattern | Count |
|---|---|
| `.skip` / `xit` / `@skip` | 0 |
| `.only` (left in) | 0 |
| `TODO: add test` / similar | 0 |
| Empty assertion / placeholder | 0 |
| `--retry` / retries normalized | no |
| Hardcoded Port / Sleep in Mocks | 1 (FTP PASV data listener port 10240) |

*Observation:* While there are no explicit skips or TODOs left in the code, the codebase still relies on a major structural shortcut: the printability and adapters crates are skeleton traits without concrete implementations, meaning the test suite runs green while 90% of the advertised engine features do not exist. Furthermore, a minor hardcoded port remains in the FTP passive mode data listener mock.

---

## Blind spots by class

- **Unimplemented Engine logic** — `printability` and `adapters` are skeletons. There are no tests for geometric checks, overhang detection, out-of-bounds G-code, or adapter protocols.
- **CLI Commands** — No integration tests verify CLI exit codes, flag behaviors, or handling of missing files.
- **Fixture integration** — The fixtures directory containing STL and G-code examples remains completely disconnected from any test.

---

## Patterns and systemic observations

- **The "Mock-Green" Test Suite (Persistent):** The codebase compiles and passes tests immediately because the developers verified serialization models but deferred implementing the core business logic. This results in 100% green test passes on code that is 0% complete.
- **Active Resolution Culture:** The recent fixes for `TEST-006` and `TEST-007` show that the development team is actively addressing test suite issues when flagged, refactoring mocks to use ephemeral ports, and expanding test coverage to previously untested mock servers.
- **Schema Generation Side-Effects (Persistent):** The schema generation is hardcoded inside a cargo test. While this ensures schemas are generated, it creates a brittle unit test suite dependent on filesystem permissions.
- **No E2E/CLI Validation:** There are no tests running the actual CLI tool, meaning command interfaces could break without failing the build.

---

## Appendix: test artifacts reviewed

- `Cargo.toml` (root, crates workspace)
- `.github/workflows/ci.yml` (CI workflow)
- `crates/core/src/lib.rs` (full source + serialization tests + schema generation test)
- `crates/printability/src/lib.rs` (full source, skeleton engine)
- `crates/adapters/src/lib.rs` (full source, skeleton adapters)
- `crates/sdk/src/lib.rs` (full source, mock server tests)
- `crates/sdk/src/mocks/` directory (mock servers)
- `crates/cli/src/main.rs` (full source, CLI app)
- `.git/hooks/pre-push` (git pre-push hook)
- `fixtures/` directory (mesh and G-code files)
- `schemas/` directory (auto-generated JSON schemas)

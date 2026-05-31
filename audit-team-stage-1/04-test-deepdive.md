# Test Suite Deep-Dive — PrintProof3D (v0.1.0)

**Audit date:** 2026-05-30
**Role:** Senior Test Engineer
**Scope audited:** Connection config validation, connection adapter implementations, connection factory, REST endpoints, and SDK mock/conformance tests in the `stage-1-connection-config-factory` branch of the `PrintProof3D` repository.
**Auditor posture:** Paranoid, adversarial.

---

## TL;DR

The central finding of this deep-dive is a classic test-suite illusion: **the tests are green, but they are checking mock client implementations rather than production code.** 

A conformance test suite in `crates/sdk/src/lib.rs` verifies connection handling, telemetry, and command dispatch, but it executes these tests against `RrfTestClient` and `BambuTestClient` stubs declared directly inside the test module. The actual production adapters (`RrfAdapter` and `BambuAdapter` in `crates/adapters`) are non-functional skeleton stubs that return "Not implemented" errors for all methods. Because the test suite only exercises the test-file stubs, the 100% passing rate is a false signal that masks completely unimplemented production code.

Furthermore, the REST API endpoints (`validate_model` and `validate_gcode`) have zero test coverage for their happy paths, and their file-handling logic contains a panic-triggered resource leak that can easily exhaust server disk space. The connection config validation lacks checks for empty/whitespace string inputs, allowing invalid configurations to pass. Finally, a unit test side effect actively overwrites workspace schema files in git on every run, violating test isolation guidelines.

---

## Severity roll-up (tests)

| Severity | Count |
|---|---|
| Blocker | 0 |
| Critical | 1 |
| Major | 3 |
| Minor | 2 |
| Nit | 0 |

---

## What's working

- **Sound Schema Invariants**: The structural validation of data profiles (`PrinterProfile`, `MaterialProfile`) is implemented cleanly in `crates/core/src/lib.rs` and has robust tests, including checks that bad volumes, unsafe temperatures, and mismatched volume/bed geometries are caught.
- **Config Invariant Rules**: Validation rules in `PrinterConnectionConfig::validate` correctly detect missing base URLs for network connections and missing serial paths for serial connections under normal input parameters.
- **WASM Memory Exchange Testing**: The WASM plugin subsystem tests in `crates/plugins/src/lib.rs` use a raw WebAssembly Text (WAT) representation, allowing memory exchange assertions to run without requiring pre-compiled external WASM binaries.

---

## What couldn't be assessed

- **CI/CD Build Performance / Real Runners**: The GitHub Actions runner behavior and caching were reviewed statically in `.github/workflows/ci.yml`. Actual runtime flakiness in CI history could not be verified due to lack of historical CI log files.
- **E2E UI Testing**: There are no frontend UI modules or E2E browser tests (Cypress/Playwright) present in this branch. Testing was assessed entirely at the crate/unit/integration level.

---

## Test landscape

| Dimension | Observation |
|---|---|
| Framework(s) | Rust standard library test framework (`cargo test`), `tokio::test` for async tests, `axum::serve` mock endpoints in tests. |
| Test pyramid shape | Bottom-heavy with unit tests in `core` and `printability`, but heavily compromised at the integration layer where adapters are mocked inside the test suites instead of being tested. No end-to-end tests exist. |
| Coverage tool | None configured. |
| Reported coverage (if any) | 100% passing tests reported by "Audit Lite", but actual production code coverage for connection configs and adapters is **0%**. |
| Flakiness posture | No retry mechanism or flakiness mitigation is present, but local `TcpListener` ports bind to `127.0.0.1:0` (ephemeral), which prevents port-collision flakiness. |
| CI blocking? | Yes, `.github/workflows/ci.yml` contains a `cargo test` job that blocks pull requests on failures. |

**Run evidence:**
- Workspace contains tests in `crates/core`, `crates/printability`, `crates/plugins`, `crates/adapters` (factory only), `crates/sdk` (mock clients only), and `crates/rest` (basic route sanity tests).

---

## Findings

### [TEST-001] — Critical — Mocking / Quality — Conformance test suite runs against custom test clients, leaving the actual production adapter code completely untested

**Evidence**
- In `crates/sdk/src/lib.rs`, the conformance test suite is defined as `run_conformance_tests<A: PrinterAdapter>(adapter: &mut A)` (lines 9-70).
- The tests `test_sdk_conformance_rrf` (lines 180-188) and `test_sdk_conformance_bambu` (lines 305-318) execute this suite, but they instantiate and pass custom local structs: `RrfTestClient` (lines 84-177) and `BambuTestClient` (lines 190-302).
- The actual production adapters under `crates/adapters/src/` (`rrf.rs`, `bambu.rs`, `moonraker.rs`, `octoprint.rs`, `prusalink.rs`, `serial.rs`) are never passed to the conformance test suite.
- The production adapters (e.g. `crates/adapters/src/bambu.rs` lines 20-70) are just skeleton stubs that return errors:
  ```rust
  async fn connect(&mut self) -> Result<(), AdapterError> {
      Err(AdapterError::ConnectionFailed("Not implemented".to_string()))
  }
  ```

**Why this matters**
The test suite compiles and runs successfully, giving developers and auditors the false impression that connection handling, telemetry, and file transfers are fully validated. In reality, the production code is completely non-functional. Any changes, refactors, or bugs introduced in the actual production adapters will never trigger a test failure because the test suite only asserts on the local, private test-file stubs.

**Blast radius**
- All 6 adapter types in `crates/adapters`.
- Conformance testing module in `crates/sdk`.

**Fix path**
The production adapters must be refactored to implement the target protocols. Once implemented, the conformance test suite must be updated to instantiate the actual production adapter structs (e.g., `BambuAdapter`, `RrfAdapter`) and verify their behavior against the mock servers, rather than testing a separate mock client.

---

### [TEST-002] — Major — Coverage — REST API endpoints for model and G-code validation lack happy-path integration tests

**Evidence**
- In `crates/rest/src/main.rs`, the `api_router()` defines routes for `/validate/model` and `/validate/gcode` (lines 308-315).
- The `tests` module in `crates/rest/src/main.rs` (lines 329-411) includes assertions for `test_list_printer_profiles`, `test_home_route`, CORS validation, and auth middleware failures (401).
- No test exists that sends a POST request with valid authorization and multipart form data containing valid printer/material profiles and model/gcode files.

**Why this matters**
The core functionality of the REST API (parsing multipart uploads, writing files to the temporary directory, running validators, and parsing reports) is completely untested. Any bugs in multipart form field parsing, serialization mismatches, or file system permissions on the server will go undetected until runtime.

**Blast radius**
- REST API service endpoints (`validate_model`, `validate_gcode`).

**Fix path**
Add integration tests in `crates/rest/src/main.rs` using Axum's `tower::ServiceExt` calling `oneshot()`. The tests should mock a multipart form request with a boundary, containing a small STL or G-code file and serialized JSON printer/material profiles, and assert that it returns `200 OK` with a valid JSON report.

---

### [TEST-003] — Major — Resource Leak / Security — Temporary uploads folder has no drop-guard cleanup on validator panic, risking disk exhaustion

**Evidence**
- In `crates/rest/src/main.rs`, the `validate_model` endpoint writes incoming file bytes to a temporary path under `temp_uploads` (line 166-167):
  ```rust
  let temp_file_path = unique_temp_file_name(&model_name);
  std::fs::write(&temp_file_path, &model_bytes)
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
  ```
- The validator is called and files are deleted afterwards (lines 170-178):
  ```rust
  let validator = StlModelValidator;
  let report = validator
      .validate_mesh(&temp_file_path, &printer, &material)
      .map_err(|e| {
          let _ = std::fs::remove_file(&temp_file_path);
          (StatusCode::INTERNAL_SERVER_ERROR, e)
      })?;

  let _ = std::fs::remove_file(&temp_file_path);
  ```
- If the validator or the underlying library panics (e.g. out of memory on huge meshes or parsing index out of bounds), the control flow bypasses both the `map_err` closure and the trailing `remove_file` statement.

**Why this matters**
Rust's AXUM handles thread panics internally by converting them to HTTP 500 errors, but the thread terminates immediately. Since there is no RAII drop-guard wrapping `temp_file_path`, the uploaded temporary file is left in the `temp_uploads` folder forever. An attacker could exploit this by uploading malformed files that cause panics, repeatedly leaking disk space until the host runs out of storage (Denial of Service).

**Blast radius**
- AXUM server host disk storage.

**Fix path**
Create a custom `TempFileGuard` struct implementing `Drop`:
```rust
struct TempFileGuard(std::path::PathBuf);
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
```
Instantiate this guard immediately after writing the file. This ensures the file is automatically deleted when the guard goes out of scope, even during a panic unwind.

---

### [TEST-004] — Major — Coverage / Quality — Connection Config Factory tests only exercise a single protocol family

**Evidence**
- In `crates/adapters/src/factory.rs`, `PrinterAdapterFactory::build` (lines 13-48) maps `ProtocolFamily` variants to their respective adapter implementations.
- The `tests` module (lines 51-135) contains only two tests: `test_factory_builds_bambu` (testing `BambuMqtt` returns `Ok`) and `test_factory_invalid_config_fails`.
- There are no tests verifying that `Klipper`, `OctoPrint`, `PrusaLink`, `RepRapFirmware`, or `MarlinSerial` build successfully via the factory.
- There are no tests verifying that unsupported protocols (like `ElegooSdcp`) return an `Unsupported protocol family` error.

**Why this matters**
The factory routing logic for 5 out of 6 supported protocols is untested. A mapping bug (e.g. instantiating the wrong adapter struct or failing on missing configuration assumptions) would go unnoticed, even though the factory is claimed to be "fully tested".

**Blast radius**
- `PrinterAdapterFactory` registry mapping logic.

**Fix path**
Add parameterized or comprehensive unit tests in `factory.rs` that attempt to build each of the supported connection configurations and assert that they return a valid boxed adapter, as well as checking that unsupported variants fail as expected.

---

### [TEST-005] — Minor — Quality / Hygiene — Unit test writes to git source tree during cargo test, violating test isolation

**Evidence**
- In `crates/core/src/lib.rs`, the `generate_schemas` test (lines 582-619) writes JSON schema files directly into the repository workspace:
  ```rust
  let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
  create_dir_all(&schema_dir).unwrap();
  // writes schemas...
  ```
- This test executes every time `cargo test` is run.

**Why this matters**
Tests should be isolated and side-effect free. Generating files directly inside the source tree during a standard test run can dirty git workspaces, cause race conditions when tests run in parallel, and fail in read-only environments (such as sandboxed CI/CD containers).

**Fix path**
Move this logic out of the standard unit test run into a separate binary tool or an explicit cargo script (e.g. `cargo run --bin generate_schemas`). In the test suite, replace it with a test that reads the schema from disk and asserts it matches the generated output, checking for schema drift without mutating the source tree.

---

### [TEST-006] — Minor — Quality / Validation — Connection config validation fails to catch empty/whitespace strings for required parameters

**Evidence**
- In `crates/core/src/connection.rs`, `PrinterConnectionConfig::validate` checks optional fields using `is_none()` (lines 95, 100, 112, 117, 123):
  ```rust
  if self.serial_path.is_none() { ... }
  ```
- No check is made on the inner string values. If a connection config is passed with empty strings (e.g., `serial_path: Some("".to_string())`), the validation logic passes successfully.

**Why this matters**
This allows empty, invalid, or whitespace-only paths and endpoints to pass safety checks, bypassing the validation rules and causing downstream library panics or silent failures when the adapters try to connect to empty endpoints.

**Blast radius**
- Path resolution and authentication credentials.

**Fix path**
Update validation checks to verify that optional string parameters, if present, are not empty or whitespace. E.g.:
```rust
if self.serial_path.as_ref().map_or(true, |s| s.trim().is_empty()) { ... }
```
Add unit tests that pass empty strings to these fields and verify they fail validation.

---

## Shortcut census

| Shortcut pattern | Count |
|---|---|
| `it.skip` / `xit` / `#[ignore]` | 0 |
| `TODO: add test` / similar comments | 0 |
| Mocks in place of production code in conformance suite | 2 (`BambuTestClient`, `RrfTestClient`) |
| Happy-path test skipped entirely due to complexity | 2 (REST endpoints) |

---

## Blind spots by class

- **Implementation Gaps Under Mock Coverage**: Testing custom mocks inside the test modules instead of actual production code, letting 100% skeleton code pass test suites (TEST-001).
- **REST Happy-Path API Integration**: Complete lack of tests for multipart validation processing and JSON serialization of reports on POST endpoints (TEST-002).
- **Abnormal Thread Unwinds (Panics)**: Lack of cleanup safeguards for disk-persisted temporary data in Axum routing logic on panics (TEST-003).
- **Validation Formats**: Checking for parameter presence (`is_none`) but ignoring string format and length constraints (`Some("")`), allowing invalid configs to validate (TEST-006).

---

## Patterns and systemic observations

- **"Mocks Lie" normalized**: The test suite exhibits a systemic anti-pattern where integration tests are run against custom test clients built inside the test files rather than the actual production code. The team seems to focus on green status indicators over verifying the real execution paths.
- **"Audit Lite" False Confidence**: A previous automated check marked the code as fully tested and ready to ship, despite the adapters being empty stubs and the REST endpoints having no happy-path coverage. This shows a culture of relying on shallow automated metrics.
- **Resource Cleanup Hygiene**: The API relies on manual deletion of files at the end of functions rather than idiomatic RAII drop guards, indicating a lack of adversarial thinking regarding failure modes, panics, or system crashes.

***

### Concise Summary for Orchestrator (Executive Report)

- **Total Finding Count:** 6 (1 Critical, 3 Major, 2 Minor, 0 Nit)
- **Blockers:** 0

**Top 5 Findings:**
1. **TEST-001 (Critical - Mocking/Quality):** Conformance test suite exercises local, custom-written `BambuTestClient` and `RrfTestClient` stubs inside `sdk/src/lib.rs` rather than the actual `BambuAdapter` and `RrfAdapter` from the `adapters` crate.
2. **TEST-002 (Major - Coverage):** Axum REST server defines route handlers `/validate/model` and `/validate/gcode` but has no happy path integration tests verifying multipart form uploads.
3. **TEST-003 (Major - Security/Leakage):** Route handlers in `rest/src/main.rs` write uploaded bytes to temporary files under `temp_uploads/`. If a validation function panics, the cleanup logic is skipped, permanently leaking files.
4. **TEST-004 (Major - Coverage):** The adapter factory unit tests in `factory.rs` only verify `BambuMqtt` instantiation, leaving the other five supported protocol families completely untested.
5. **TEST-005 (Minor - Isolation):** The `generate_schemas` test in `crates/core/src/lib.rs` writes directly to `../../schemas` during a normal `cargo test` run, violating test isolation and dirtying git workspaces.

**Culture/Pattern Observations:**
- **False Green Signal:** The test suite reports 100% success by testing local mock implementations that mimic the protocol inside the test modules, hiding the fact that the actual production adapters are non-functional stubs.
- **Lack of Defensive/Adversarial Code:** Resource cleanup does not use RAII drop guards, and string inputs in configurations are checked with basic `is_none()` instead of sanitizing/checking for empty/whitespace formatting, creating security vulnerabilities (disk exhaustion DoS) and configuration bypasses.
- **Automated Validation Complacency:** A previous "Audit Lite" concluded the codebase was fully validated and ready to ship, highlighting a pattern of accepting passing test suites without checking what they actually cover.

# Test Suite Deep-Dive — PrintProof3D

**Audit date:** 2026-05-30
**Role:** Senior Test Engineer
**Scope audited:** Workspace sub-crates (`core`, `printability`, `adapters`, `sdk`, `cli`), fixtures (`fixtures/` directory), schemas (`schemas/` directory), Git hooks (`.git/hooks/pre-push`), and cargo dependency configurations.
**Auditor posture:** Adversarial (professionally paranoid test engineering perspective)

---

## TL;DR

The PrintProof3D test suite is currently a "passing skeleton" that provides a false sense of security. While the cargo test command runs instantly and reports 100% success (4 tests passed), this is only because the core business logic is completely unwritten and represented by dummy placeholders (e.g. returning static `"ok"` or `"initialized"` strings). The entire test suite consists of 3 happy-path serialization roundtrips and 1 side-effecting schema generator, leaving 100% of the project's claimed features (mesh validation, boundary checking, connection adapters, developer SDK, CLI flags/commands) completely untested. Additionally, a directory of 3D-printing test fixtures (STLs, G-code files) exists in the repo but is entirely unused by any test.

## Severity roll-up (tests)

| Severity | Count |
|---|---|
| Blocker | 1 |
| Critical | 1 |
| Major | 2 |
| Minor | 1 |
| Nit | 0 |

## What's working

- **Model Roundtrip Serialization** — `crates/core/src/lib.rs:170-266` successfully asserts that `PrinterProfile`, `MaterialProfile`, and `ValidationReport` correctly roundtrip serialize and deserialize to JSON.
- **Pre-Push Gate Scripting** — The local git hook at `.git/hooks/pre-push` is correctly configured to run `cargo test --workspace` and block pushes if the compilation or test run fails.

## What couldn't be assessed

- **CI History and Flakiness** — Since there is no automated CI pipeline configured (e.g., GitHub Actions), there is no CI history or build log log to assess flaky test rates.
- **Coverage Tooling** — No code coverage tool (e.g., cargo-tarpaulin or llvm-cov) is configured, though due to the lack of actual code, the functional coverage is effectively 0%.

---

## Test landscape

| Dimension | Observation |
|---|---|
| Framework(s) | Rust standard testing framework (`cargo test`) |
| Test pyramid shape | Sparse Unit (4 tests total) / No integration tests / No E2E tests |
| Coverage tool | None configured |
| Reported coverage (if any) | None (actual functional coverage is 0%) |
| Flakiness posture | Clean (due to lack of async, I/O, or concurrent code in tests) |
| CI blocking? | No (No CI environment exists; local git pre-push hook only) |

---

## Findings

> **Finding ID prefix:** `TEST-`
> **Categories:** Coverage / Shortcut / Flakiness / Quality / Ergonomics / Mocking / Regression / CI

### [TEST-001] — Blocker — Quality / Coverage — Skeleton Implementation with 100% Dummy Success and Zero Feature Tests

**Evidence**
- `crates/printability/src/lib.rs` (lines 3-5):
  ```rust
  pub fn check_model() -> &'static str {
      "ok"
  }
  ```
- `crates/adapters/src/lib.rs` (lines 3-5):
  ```rust
  pub fn list_adapters() -> Vec<&'static str> {
      vec!["moonraker", "octoprint", "marlin"]
  }
  ```
- `crates/sdk/src/lib.rs` (lines 3-5):
  ```rust
  pub fn sdk_init() -> &'static str {
      "initialized"
  }
  ```
- `crates/cli/src/main.rs` (lines 3-5):
  ```rust
  fn main() {
      println!("PrintProof3D CLI version 0.1.0");
  }
  ```
- Zero test definitions exist inside `crates/printability`, `crates/adapters`, `crates/sdk`, or `crates/cli`.

**Why this matters**
The project `README.md` asserts that the repository implements mesh validation algorithms, G-code static boundary checkers, core connection adapters (OctoPrint, Moonraker, Marlin), and a developer SDK. However, these crates are empty skeletons. The test suite passes 100% successfully because it only runs serialization checks on a few data structures in the `core` crate. Any developer or consumer of this project would run `cargo test --workspace` and receive a "green" status, yet the engine does not perform any of its advertised logic. This masks a complete lack of functional application behavior and creates a dangerous blind spot.

**Blast radius**
- Workspace crates: `printproof3d-printability`, `printproof3d-adapters`, `printproof3d-sdk`, `printproof3d` (CLI).

**Fix path**
Implement the actual parsing, geometry validation, and adapter logic in their respective crates. Create dedicated unit and integration tests inside each crate that exercise real inputs, edge cases, and protocol interactions.

---

### [TEST-002] — Critical — CI — Complete Absence of Automated CI/CD Pipelines

**Evidence**
- The workspace root has no `.github` directory or CI configuration files (e.g., GitHub Actions, GitLab CI, CircleCI, etc.).
- The only verification gating is the local `.git/hooks/pre-push` hook which runs `cargo test --workspace`.

**Why this matters**
Relying exclusively on local git hooks to ensure quality is a severe risk. Local hooks are easily bypassed (e.g., `git push --no-verify`), do not run on code committed via the GitHub web editor, do not execute on pull requests from forks, and depend entirely on the host developer's environment (which may have different rustc versions or target environments). This allows broken, untested, or unformatted code to be merged into the main branch undetected, rendering the local test suite check toothless as a gateway control.

**Blast radius**
- Entire repository and development pipeline.

**Fix path**
Scaffold a CI workflow (e.g., `.github/workflows/ci.yml`) that triggers on every push and pull request to `main`/`master` branches. The workflow should run `cargo check`, `cargo test --workspace`, and `cargo clippy --all-targets -- -D warnings` on a clean, isolated runner.

---

### [TEST-003] — Major — Coverage / Quality — Test Fixture Library is Completely Disconnected and Unused

**Evidence**
- The root `fixtures/` directory contains files like `open_triangle.stl` (185 bytes), `overhang_flange.stl` (192 bytes), `tetrahedron.stl` (631 bytes), `out_of_bounds.gcode` (285 bytes), `safe_print.gcode` (479 bytes), and `unsafe_temp.gcode` (212 bytes).
- A workspace-wide grep search for `fixtures`, `open_triangle`, `overhang_flange`, `tetrahedron`, `out_of_bounds`, `safe_print`, and `unsafe_temp` yields zero occurrences in any `.rs` file.

**Why this matters**
The presence of this fixture library suggests that the engine is tested against realistic mesh files (manifold and non-manifold) and G-code sequences (safe, out-of-bounds, unsafe temperatures). In reality, these fixtures are dead weight. No parser is reading these files, meaning changes to the file format, parser implementation, or structural models will not trigger test failures. This is a severe coverage gap where edge cases are documented as fixtures but never exercised in code.

**Blast radius**
- `printability` crate and mesh analysis paths.

**Fix path**
Add integration tests in `crates/printability` that read these fixtures from disk using relative paths (e.g., `std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/...")`), parse them, and assert that the printability engine flags the non-manifold mesh (`open_triangle.stl`), detects the overhang issue, and correctly identifies out-of-bounds/unsafe G-code blocks.

---

### [TEST-004] — Major — Quality — Static Happy-Path Model/Material Profile Verification without Boundary Checking

**Evidence**
- In `crates/core/src/lib.rs`, the serialization tests (`test_printer_profile_serialization`, `test_material_profile_serialization`, `test_validation_report_serialization` at lines 170-267) construct valid instances of structures with static fields and check that they roundtrip serialize/deserialize successfully.
- There are no tests checking bounds or constraints on these structs (e.g., negative nozzle diameters, negative build volume boundaries, maximum bed temperatures exceeding physical limits, or empty abbreviations vectors).

**Why this matters**
Data models are deserialized from user-supplied profile JSONs. If user inputs are not constrained and validated at the boundary, bad data (e.g., a build volume of `[0.0, 0.0, 0.0]` or a negative nozzle diameter) will be deserialized without error, downstream modules will crash (e.g., divide-by-zero during slicing calculations), or invalid parameters will be sent to the physical printer.

**Blast radius**
- `printproof3d-core` and downstream validation logic.

**Fix path**
Implement a `validate()` method or use crate validator attributes on the data structures (e.g., `PrinterProfile`, `MaterialProfile`) to verify constraints. Add unit tests verifying that invalid configurations (like zero nozzle diameters or negative build volume dimensions) are caught and rejected during validation.

---

### [TEST-005] — Minor — Ergonomics / CI — Schema Generation Side-Effects Inside Cargo Unit Test

**Evidence**
- In `crates/core/src/lib.rs` (lines 268-290), the `generate_schemas()` test writes JSON schema files to the local filesystem:
  ```rust
  let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
  create_dir_all(&schema_dir).unwrap();
  // ... writes printer_profile.schema.json, material_profile.schema.json, validation_report.schema.json
  ```

**Why this matters**
Unit tests should be pure, idempotent, and side-effect free. Writing files to the workspace during test execution violates this principle. If the test runs in a read-only or sandboxed environment (which is typical for secure CI runners), the test will fail. Additionally, concurrent test execution can cause file locking conflicts, especially on Windows. A unit test runner should not double as a build automation or code-generation tool.

**Blast radius**
- `printproof3d-core` unit tests.

**Fix path**
Move the schema generation logic out of the cargo unit tests. Instead, create a separate bin target (e.g., `crates/core/src/bin/generate_schemas.rs`) or a `build.rs` script that is run explicitly when schemas need to be regenerated, leaving `cargo test` clean and side-effect free.

---

## Shortcut census

| Shortcut pattern | Count |
|---|---|
| `.skip` / `xit` / `@skip` | 0 |
| `.only` (left in) | 0 |
| `TODO: add test` / similar | 0 |
| Empty assertion / placeholder | 0 |
| `--retry` / retries normalized | no |

*Observation:* While there are no explicit skips or TODOs left in the code, the entire repository represents a structural shortcut: implementing empty skeleton functions with green tests to pass initial checks, rather than writing actual code and tests.

---

## Blind spots by class

- **Functional validation logic** — Since the `printability` and `adapters` engines are unimplemented placeholders, any geometry parsing or connection adapter logic is a complete blind spot.
- **Malformed profile inputs** — No verification of boundary limits, physical properties (e.g. negative dimensions, division by zero parameters), or string validation for profile structures.
- **Fixture integration** — The static fixture files (STL and G-code) are never loaded, leaving mesh parse verification completely untested.
- **Command-line execution** — The CLI crate lacks clap argument parsing and lacks any test asserting correct exit codes, error outputs, or behavior on commands.
- **Concurrency & I/O** — No tests verify file reading, network sockets, or concurrent requests in the adapter layer.

---

## Patterns and systemic observations

- **The "Mock-Green" Test Suite:** The codebase compiles and passes tests immediately because the developers verified serialization models but deferred implementing the core business logic. This results in 100% green test passes on code that is 0% complete.
- **Task Automation via Tests:** The schema generation is hardcoded inside a cargo test. While this ensures schemas are generated, it creates a brittle unit test suite dependent on filesystem permissions.
- **No CI/CD Guardrails:** The complete lack of CI files indicates that quality gating is offloaded entirely to local machine environments.

---

## Appendix: test artifacts reviewed

- `Cargo.toml` (root, crates workspace)
- `crates/core/src/lib.rs` (full source + serialization tests + schema generation test)
- `crates/printability/src/lib.rs` (full source, skeleton engine)
- `crates/adapters/src/lib.rs` (full source, skeleton adapters)
- `crates/sdk/src/lib.rs` (full source, skeleton SDK)
- `crates/cli/src/main.rs` (full source, skeleton CLI)
- `.git/hooks/pre-push` (git pre-push hook)
- `fixtures/` directory (mesh and G-code files)
- `schemas/` directory (auto-generated JSON schemas)

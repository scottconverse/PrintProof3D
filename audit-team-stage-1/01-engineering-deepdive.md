# Engineering Deep-Dive — PrintProof3D (printproof3d v0.1.0)

**Audit date:** 2026-05-30
**Role:** Principal Engineer
**Scope audited:** Full workspace scaffolding at `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D`, including crates (`core`, `printability`, `adapters`, `sdk`, `cli`), configurations, schemas, and fixtures.
**Auditor posture:** Balanced / Professional

---

## TL;DR

The Stage 1 workspace has a clean layout and compiles successfully. However, the implementation is currently in a "skeleton" phase. The `core` crate contains the data models and auto-schema generation, while the other four crates (`printability`, `adapters`, `sdk`, `cli`) consist of stub files with dummy functions. 

The audit highlights a **Critical** safety concern regarding the lack of range/bounds validation for user-uploaded printer/material profile files (which could specify negative dimensions or hazardous temperatures). It also reveals **Major** architectural debt: the lack of common traits/interfaces for printer adapters and validators will lead to high coupling as implementation begins, and executing schema auto-generation inside unit tests creates workspace side-effects.

---

## Severity roll-up (engineering)

| Severity | Count |
|---|---|
| Blocker | 0 |
| Critical | 1 |
| Major | 3 |
| Minor | 2 |
| Nit | 2 |

---

## What's working

- **Workspace Architecture**: The Cargo workspace is well-structured, allowing `core`, `printability`, `adapters`, `sdk`, and `cli` to build together seamlessly.
- **Serialization and Schemas**: The data models in `crates/core` correctly derive `Serialize`, `Deserialize`, and `JsonSchema`, enabling roundtrip validation checks.
- **Pre-Push Validation**: The pre-push hook configuration is correctly established to gate broken builds/tests prior to commits.
- **Test Coverage for Core**: Basic roundtrip serialization unit tests are present and passing for `PrinterProfile`, `MaterialProfile`, and `ValidationReport`.

---

## What couldn't be assessed

- **Runtime Connection Adapters**: Code for connecting to Moonraker, OctoPrint, or Marlin serial consists entirely of stub functions returning mock strings; real I/O and communication could not be evaluated.
- **Mesh and G-Code Checking**: The boundary checkers and mesh checks are stubs (`check_model() -> &'static str { "ok" }`). No physical STL loading or G-code parsing could be evaluated.
- **CLI and MCP Server**: The CLI entry point simply prints a version number and has no argument parser or MCP server implementation.

---

## Findings

### [ENG-001] — Critical — Correctness & Security — Lack of safety and range validation on Profile deserialization

**Evidence**
`crates/core/src/lib.rs:56-85` (`PrinterProfile`) and `96-111` (`MaterialProfile`) define properties such as nozzle diameters, temperatures, and layer heights:
```rust
pub struct PrinterProfile {
    ...
    pub nozzle_diameters: Vec<f32>,
    pub default_nozzle_diameter: f32,
    pub min_layer_height: f32,
    pub max_layer_height: f32,
    pub max_hotend_temp: f32,
    pub max_bed_temp: f32,
    ...
}
```
There is no validation check applied during deserialization or instantiation to ensure:
1. Coordinates and nozzle dimensions are positive (e.g. `default_nozzle_diameter` or `max_layer_height` are not zero or negative).
2. Temperatures are within physically safe limits (e.g., preventing a hotend limit of 10,000°C or negative bed temperatures).
3. Logical relationships hold (e.g., `min_layer_height <= max_layer_height`, and `default_nozzle_diameter` is present in the `nozzle_diameters` array).

**Why this matters**
These profiles represent untrusted inputs (user uploads or third-party database syncs). Without strict bounds sanitization:
1. Division-by-zero errors or panic indexing can occur inside slicing/verification calculations if nozzle size or layer heights are set to zero/negative values.
2. Underflow/overflow bugs could bypass print-safety checks.
3. In extreme cases, if downstream hardware controllers rely on these values without validation, sending extreme temperature targets to a printer can cause hardware damage or thermal runaway hazards.

**Blast radius**
- **Adjacent code**: `crates/core/src/lib.rs` (schema structs), and any future printability validation logic.
- **Shared state**: Deserialized models in memory.
- **User-facing**: Profile configuration JSONs.
- **Migration**: Update struct schemas with validation annotations or introduce custom deserialization invariants.
- **Tests to update**: Add validation tests with malformed inputs in `crates/core/src/lib.rs`.
- **Related findings**: ENG-007.

**Fix path**
Implement a validation check (e.g. by implementing the `Validate` trait or custom deserializers) that checks all physical dimensions and temperatures against sane bounds (e.g. `nozzle_diameter > 0.0`, `max_hotend_temp <= 500.0`, `min_layer_height <= max_layer_height`, and `nozzle_diameters.contains(&default_nozzle_diameter)`). Reject deserialization if these rules are violated.

---

### [ENG-002] — Major — Architecture — Lack of Abstraction Traits for Printer Adapters and Printability Validators

**Evidence**
- `crates/adapters/src/lib.rs:3-5` defines a static list of strings and no traits:
```rust
pub fn list_adapters() -> Vec<&'static str> {
    vec!["moonraker", "octoprint", "marlin"]
}
```
- `crates/printability/src/lib.rs:3-5` defines a static stub:
```rust
pub fn check_model() -> &'static str {
    "ok"
}
```

**Why this matters**
Currently, there are no common traits defining how connection adapters or printability validators should behave. As developers start writing Klipper, OctoPrint, and Marlin implementations, they will likely create diverging API interfaces. This lack of common abstraction layers will lead to highly coupled code in the SDK and CLI crates, making future support for additional protocols difficult and forcing major refactorings.

**Blast radius**
- **Adjacent code**: `crates/adapters`, `crates/printability`, `crates/sdk`, and `crates/cli`.
- **User-facing**: The developer experience for implementing or extending PrintProof3D.
- **Migration**: Massive refactoring risk if implementation proceeds without interface definition.
- **Tests to update**: Introduce mock adapter tests once traits are defined.

**Fix path**
Define clear interfaces using Rust traits. For example, in `crates/adapters`, define a `PrinterAdapter` trait:
```rust
#[async_trait]
pub trait PrinterAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError>;
    async fn upload_file(&self, path: &Path) -> Result<(), AdapterError>;
    async fn get_status(&self) -> Result<PrinterStatus, AdapterError>;
}
```
And in `crates/printability`, define a `ModelValidator` or `GcodeValidator` trait.

---

### [ENG-003] — Major — Performance / Correctness — Schema Auto-Generation executed inside Unit Tests creates Workspace Side-Effects

**Evidence**
`crates/core/src/lib.rs:269-291` (`generate_schemas`) uses `cargo test` execution to write schema files directly to the root repository folder:
```rust
    #[test]
    fn generate_schemas() {
        ...
        let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
        create_dir_all(&schema_dir).unwrap();
        ...
```

**Why this matters**
Unit tests are expected to be hermetic, side-effect-free, and safe to execute concurrently in any environment.
1. Writing files to the workspace workspace directory during a test run breaks test hermeticity.
2. In restricted or read-only CI/CD execution environments, this test will fail due to lack of write permissions.
3. If multiple tests or workspace builds are executed in parallel, it can lead to file lock contention or race conditions on the schema files.

**Blast radius**
- **Adjacent code**: `crates/core/src/lib.rs` (unit tests).
- **Shared state**: The `/schemas` directory at the workspace root.
- **User-facing**: Developer local environment and CI/CD pipelines.
- **Migration**: Move the generator logic out of the test harness.

**Fix path**
Remove the file-writing logic from the unit test. Instead, create a dedicated cargo binary target (e.g., a generator script under `crates/core/src/bin/generate_schemas.rs`) or use a `cargo-xtask` layout so that schema generation is an explicit developer action rather than a side-effect of running unit tests.

---

### [ENG-004] — Major — Correctness — Missing State Invariant Enforcement in Validation Reports

**Evidence**
`ValidationReport` in `crates/core/src/lib.rs:156-164` exposes all fields publicly without logical validation constraints:
```rust
pub struct ValidationReport {
    pub status: ValidationStatus,
    ...
    pub issues: Vec<ValidationIssue>,
    ...
}
```

**Why this matters**
Because the fields are fully public and there is no constructor or validation helper, a report can be constructed or deserialized with inconsistent state. For example, a report can have its `status` set to `ValidationStatus::Pass` while containing multiple issues of severity `IssueSeverity::Blocker` or `Critical`. If downstream integration software (e.g. print queues) relies on the `status` field to allow or deny a print job, this invariant bypass could allow unsafe prints to start.

**Blast radius**
- **Adjacent code**: `crates/core/src/lib.rs` (struct definitions).
- **Shared state**: Serialized validation reports stored or transmitted across services.
- **Migration**: Transition the fields to be read-only/private and instantiate reports via constructor logic.
- **Tests to update**: Unit tests in `crates/core` verifying that reports with blockers cannot pass validation.

**Fix path**
Provide an invariant-enforcing constructor or a `.validate()` method on `ValidationReport` that enforces correctness. For example, assert that if `issues` contains any issue with a severity of `Blocker` or `Critical`, the `status` must be `ValidationStatus::Fail`.

---

### [ENG-005] — Minor — Security — Unsanitized User-Controlled Filenames in ModelMetadata / Path Traversal Risk

**Evidence**
`ModelMetadata` in `crates/core/src/lib.rs:132-136` stores a user-supplied model filename directly:
```rust
pub struct ModelMetadata {
    pub file_name: String,
    ...
}
```

**Why this matters**
`file_name` is taken directly from the uploaded model. If downstream code (such as the adapter or CLI) writes validation reports to disk or stores cache files using this filename without sanitizing path characters, an attacker could supply a filename containing path traversal sequences (e.g., `../../etc/passwd` or `..\..\System32\...`), leading to an Arbitrary File Write / Path Traversal vulnerability.

**Fix path**
Ensure that whenever `file_name` is used in filesystem operations, it is sanitized to remove path separators (`/`, `\`) and traversal sequences (`..`). Alternatively, store the sanitized path in the struct or enforce sanitization at the input boundary.

---

### [ENG-006] — Minor — Performance — High Memory Allocation Overhead on Validation Issues

**Evidence**
`ValidationIssue` in `crates/core/src/lib.rs:147-153` allocates owned strings for every error:
```rust
pub struct ValidationIssue {
    pub id: String,
    pub severity: IssueSeverity,
    pub message: String,
    pub location: Option<IssueLocation>,
    pub suggested_fixes: Vec<String>,
}
```

**Why this matters**
When validating complex models or long G-code files, the validation engine may identify thousands of small violations (e.g., individual overhang points or minor speed violations). Storing every single issue with owned `String` and `Vec<String>` fields creates high heap allocation overhead.

**Fix path**
Use structured error codes (e.g., `OverhangTooSteep`) instead of repeating identical error strings. Resolve user-friendly messages and suggested fixes in the CLI or frontend via a translation dictionary, rather than allocating strings per issue instance. Alternatively, use `Cow<'static, str>` or reference structures to reduce allocations.

---

### [ENG-007] — Nit — Correctness / Security — Unvalidated Regex Pattern in filename_restrictions

**Evidence**
`PrinterProfile::filename_restrictions` in `crates/core/src/lib.rs:84` is stored as an unvalidated `Option<String>`.

**Why this matters**
If this string represents a regular expression used to validate files uploaded to a printer, loading a profile with an invalid regex pattern will cause the application to panic or crash when compiling it at runtime. It also opens up potential Denial of Service (ReDoS) surfaces if the pattern is malicious.

**Fix path**
Validate the regex string during deserialization of the `PrinterProfile` (e.g., by attempting to parse/compile it with the `regex` crate) and reject the profile if it is invalid.

---

### [ENG-008] — Nit — Hygiene — Unused Chrono Dependency

**Evidence**
`crates/core/Cargo.toml:10-11` specifies `chrono` as a dependency:
```toml
chrono = { version = "0.4", features = ["serde"] }
```
However, no types or functions from `chrono` are imported or used in `crates/core/src/lib.rs`.

**Why this matters**
Declaring unused dependencies increases compilation times, bloats the cargo dependency tree, and introduces unnecessary maintenance and CVE tracking surface area.

**Fix path**
Remove `chrono` (and its reference in the `schemars` features) from `crates/core/Cargo.toml` if date/time calculations are not required in the core schema.

---

## Patterns and systemic observations

- **Scaffolding State**: The current codebase acts as a scaffold rather than an operating application. While this is normal for a Stage 1 release, it is the highest-leverage time to establish robust traits and interfaces (as detailed in ENG-002) before implementation code commits developers to concrete API designs.
- **Lack of Defensive Input Validation**: The schema models rely heavily on the type system to serialize and deserialize data, but do not validate the logical correctness of the fields themselves. Adding defensive validation early prevents boundary-crossing errors at runtime.

---

## Dependency snapshot

Third-party dependencies utilized in the workspace:

| Dependency | Version (Lockfile) | Context & License | Concern |
|---|---|---|---|
| `serde` | 1.0.228 | Serialization framework (MIT/Apache-2.0) | None |
| `serde_json` | 1.0.150 | JSON serializer/deserializer (MIT/Apache-2.0) | None |
| `schemars` | 0.8.22 | JSON Schema generator (MIT) | None |
| `clap` | 4.6.1 | Command-line argument parser (MIT/Apache-2.0) | None |
| `chrono` | 0.4.44 | Date/Time library (MIT/Apache-2.0) | Unused (see ENG-008) |

---

## Appendix: artifacts reviewed

- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\Cargo.toml`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\Cargo.lock`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\core\Cargo.toml`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\core\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\printability\Cargo.toml`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\printability\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\adapters\Cargo.toml`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\adapters\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\sdk\Cargo.toml`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\sdk\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\cli\Cargo.toml`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\cli\src\main.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\README.md`

# Engineering Deep-Dive — PrintProof3D (printproof3d v0.1.0)

**Audit date:** 2026-05-30
**Role:** Principal Engineer
**Scope audited:** Full workspace scaffolding at `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D`, including crates (`core`, `printability`, `adapters`, `sdk`, `cli`), configurations, schemas, and fixtures.
**Auditor posture:** Balanced / Professional

---

## TL;DR

This Stage 1 workspace update shows progress in resolving previous design deficiencies. Key improvements include defining stable core traits for printer adapters (`PrinterAdapter`) and validators (`ModelValidator`, `GcodeValidator`), introducing cylindrical bed support via the `BuildVolume` tagged enum, and adding dynamic port allocation for mock servers in tests.

However, several critical and major issues remain unresolved or only partially addressed:
- Range and bounds validation (`.validate()`) is manual and not automatically run during profile deserialization, meaning invalid profiles can still be loaded in memory.
- State invariants in validation reports are similarly checked manually rather than enforced during construction.
- The schema auto-generation code remains embedded as a cargo unit test, creating side-effects on the filesystem when running `cargo test`.
- Unsanitized path-traversal strings can still load into `ModelMetadata` if parsed outside the CLI.
- The mock servers for Bambu FTP and MQTT introduce blocking TCP stream reads that run synchronously on the mock thread, causing deadlocks, preventing clean server shutdowns, and causing parallel test processes to leak threads.
- The Bambu FTP mock's passive mode now spawns a data connection thread, but it remains hardcoded to port 10240 and lacks timeouts, leading to potential port collisions and thread leaks.

---

## Severity roll-up (engineering)

| Severity | Count (Active) | Status Roll-up |
|---|---|---|
| Blocker | 0 | 0 Active |
| Critical | 2 | 2 Partially Resolved |
| Major | 3 | 2 Resolved, 1 Partially Resolved, 2 Unresolved |
| Minor | 2 | 1 Resolved, 1 Partially Resolved, 1 Unresolved |
| Nit | 2 | 2 Unresolved |
| **Total** | **9** | **3 Fully Resolved, 4 Partially Resolved, 5 Unresolved** |

---

## What's working

- **Workspace Architecture**: The Cargo workspace compiles cleanly, with all sub-crates (`core`, `printability`, `adapters`, `sdk`, `cli`) integrated.
- **Trait-Based Interfaces**: Stable traits like `PrinterAdapter`, `ModelValidator`, and `GcodeValidator` are defined, preventing coupling.
- **Circular Bed Support**: The `BuildVolume` tagged enum (`Rectangular` vs `Cylindrical`) correctly supports cylindrical/delta beds.
- **Mock Port Allocation**: Mock servers bind to port `0` and use `local_addr().unwrap().port()` for dynamic port assignment, resolving test port collision issues.
- **Buffer Index Verification**: The Bambu MQTT mock now validates buffer length (`if n >= 4`) before indexing, resolving out-of-bounds reads.

---

## What couldn't be assessed

- **Production Network Connections**: Live connection protocols to external hosts (Moonraker, OctoPrint, Marlin) remain unimplemented stubs.
- **Physical Slicing/Validation Engines**: Mesh validation and G-code analysis engines remain mock stubs.
- **Model Context Protocol (MCP) Server**: No MCP server implementation for integration with LLM agents was visible.

---

Finds...

## Findings

### [ENG-001] — Critical — Correctness & Security — Profile Range/Bounds Validation is Manual and Not Automatic on Deserialization (PARTIALLY RESOLVED)

**Evidence**
- `crates/core/src/lib.rs:128-186` (`PrinterProfile::validate()`)
- `crates/core/src/lib.rs:230-256` (`MaterialProfile::validate()`)

**Why this matters**
While validation checks have been implemented via `.validate()` and are called inside the CLI binary, they are not automatically run during deserialization. If downstream systems or external tools deserialize profiles using `serde_json::from_str` or `from_reader` without explicitly calling `.validate()` afterwards, invalid values (like negative dimensions or 600°C nozzle limits) will still load into memory. In addition, the maximum temperature checks (500°C for hotend, 200°C for bed) remain hardcoded, and the validations are not checked at the serialization/deserialization boundary.

**Blast radius**
- **Adjacent code**: `crates/core` schema structs and third-party consumers.
- **User-facing**: Profile configuration JSONs.

**Fix path**
Implement a custom `serde::Deserialize` or a validation wrapper (such as using a crate like `validator` or implementing `TryFrom` / helper structs) that forces validation during JSON parsing, preventing invalid configurations from ever being instantiated in memory.

---

### [ENG-002] — Major — Architecture — Lack of Abstraction Traits for Printer Adapters and Printability Validators (RESOLVED)

**Evidence**
- `crates/adapters/src/lib.rs:32-43` (`PrinterAdapter` trait)
- `crates/printability/src/lib.rs:9-25` (`ModelValidator` and `GcodeValidator` traits)

**Status**
Fully resolved. The traits now form a stable interface layer.

---

### [ENG-003] — Major — Performance / Correctness — Schema Auto-Generation executed inside Unit Tests creates Workspace Side-Effects (UNRESOLVED)

**Evidence**
- `crates/core/src/lib.rs:493-516` (`generate_schemas` unit test)

**Why this matters**
Running `cargo test` still writes schema files directly to the root `../../schemas` directory. This creates file side-effects during testing, violates test hermeticity, causes file lock contention in parallel testing environments, and will fail in read-only CI pipelines.

**Blast radius**
- **Adjacent code**: `crates/core/src/lib.rs` (unit tests).
- **Shared state**: The `/schemas` directory at the workspace root.

**Fix path**
Move the schema generation code into a dedicated binary target (e.g. `crates/core/src/bin/generate_schemas.rs`) or a `cargo xtask` script so that it is invoked explicitly by developers rather than running as a side-effect of `cargo test`.

---

### [ENG-004] — Major — Correctness — Missing State Invariant Enforcement in Validation Reports (PARTIALLY RESOLVED)

**Evidence**
- `crates/core/src/lib.rs:364-376` (`ValidationReport::validate()`)

**Why this matters**
A validation check `.validate()` was added to enforce that a report with `Blocker` or `Critical` issues cannot have `ValidationStatus::Pass`. However, as with ENG-001, this validation must be manually invoked by developers. A developer could still instantiate or deserialize a report with inconsistent state without triggering an error, and the fields of `ValidationReport` remain public.

**Blast radius**
- **Adjacent code**: `crates/core/src/lib.rs` (struct definitions).

**Fix path**
Transition `ValidationReport` fields to be private or access-controlled, and enforce this constraint inside a constructor function or custom deserializer.

---

### [ENG-005] — Minor — Security — Unsanitized User-Controlled Filenames in ModelMetadata / Path Traversal Risk (PARTIALLY RESOLVED)

**Evidence**
- `crates/cli/src/main.rs:117` and `194`

**Why this matters**
The CLI has been updated to extract the base name of files using `.file_name()`, which successfully prevents path traversal when invoking the CLI. However, if `ModelMetadata` is deserialized from JSON in other contexts, the `file_name` field remains a raw, unvalidated `String` that could contain directory traversal elements (`../`).

**Fix path**
Sanitize the `file_name` property directly within the struct validation method or during deserialization.

---

### [ENG-006] — Minor — Performance — High Memory Allocation Overhead on Validation Issues (UNRESOLVED)

**Evidence**
- `crates/core/src/lib.rs:331-344` (`ValidationIssue`)

**Why this matters**
The struct still allocates owned `String` and `Vec<String>` fields for every violation. In large models or long G-code files with thousands of small alerts, this will lead to high heap allocation overhead.

**Fix path**
Use structured enum error codes or static error messages rather than owned strings, or utilize `Cow<'static, str>` to reuse common descriptions.

---

### [ENG-007] — Nit — Correctness / Security — Unvalidated Regex Pattern in filename_restrictions (UNRESOLVED)

**Evidence**
- `crates/core/src/lib.rs:125` (`filename_restrictions: Option<String>`)

**Why this matters**
The `PrinterProfile::validate()` method does not compile or check the regex string, meaning invalid regex patterns will cause runtime errors or panics when compiled later.

**Fix path**
Import the `regex` crate and attempt to compile `filename_restrictions` during validation, rejecting profiles with malformed regex.

---

### [ENG-008] — Nit — Hygiene — Unused Chrono Dependency (UNRESOLVED)

**Evidence**
- `crates/core/Cargo.toml:10-11`

**Why this matters**
`chrono` is still declared as a dependency and enabled in `schemars` features, despite not being used in `lib.rs`. This increases build times and CVE tracking surface area.

**Fix path**
Remove `chrono` from `crates/core/Cargo.toml`.

---

### [ENG-009] — Major — Correctness / Performance — Synchronous Blocking Reads in SDK Mock Servers (UNRESOLVED)

**Evidence**
- `crates/sdk/src/mocks/rrf.rs:26`
- `crates/sdk/src/mocks/bambu.rs:27`, `109`

**Why this matters**
The mock servers accept streams, set them to blocking mode, and then execute synchronous `read()` calls on the server thread. For `BambuFtpMock` and `BambuMqttMock`, this blocks the background thread indefinitely if the client keeps the connection open without sending data. Consequently, calling `.stop()` does not shut down the server thread cleanly, causing resource leaks and thread/port lockups in parallel test environments.

**Blast radius**
- **Adjacent code**: `crates/sdk/src/mocks/`
- **Shared state**: Mock background threads.

**Fix path**
Set read timeouts on the TCP stream using `stream.set_read_timeout(Some(Duration::from_millis(500)))` or use non-blocking/async-based I/O.

---

### [ENG-010] — Critical — Correctness — Bambu FTP Mock PASV Mode does not listen on the Data Port (PARTIALLY RESOLVED)

**Evidence**
- `crates/sdk/src/mocks/bambu.rs:40-52`

**Why this matters**
The FTP mock has been updated to spawn a background thread and bind a listener when the `PASV` command is received. However, this port remains hardcoded to `10240`. If multiple PASV commands are executed concurrently, subsequent attempts to bind to port 10240 will fail. In addition, the spawned thread calls `data_listener.accept()` synchronously without any timeout, meaning if the client never establishes a data connection, the thread will leak indefinitely.

**Blast radius**
- **Adjacent code**: `crates/adapters/src/` or `crates/sdk/src/` mock uploads.

**Fix path**
Dynamically bind a `TcpListener` to port `0` when `PASV` is received, retrieve the OS-assigned port via `local_addr().unwrap().port()`, translate that port into FTP-compatible octets for the `227` response, and enforce a connection/read timeout on the data connection.

---

### [ENG-011] — Major — Correctness — Unvalidated Buffer Indexing in Bambu MQTT Mock (RESOLVED)

**Evidence**
- `crates/sdk/src/mocks/bambu.rs:123-131`

**Why this matters**
Previously, the code did not verify if the read byte length `n` was at least 4 before extracting packet details, creating risk of panic or reading stale buffer data.

**Status Update**
Fully resolved. The code now checks `if n >= 4` before indexing `buffer[2]` and `buffer[3]`.

---

### [ENG-012] — Minor — Hygiene — Hardcoded Ports in SDK Unit Tests (RESOLVED)

**Evidence**
- `crates/sdk/src/mocks/rrf.rs:17-18`
- `crates/sdk/src/mocks/bambu.rs:17-18`, `92-93`

**Why this matters**
Previously, tests used hardcoded ports (18898, 18899), creating potential port collision conflicts.

**Status Update**
Fully resolved. Tests and mock servers now bind to port `0` and resolve dynamic ports at runtime.

---

## Patterns and systemic observations

- **Validation Lifecycle**: Helper methods like `.validate()` are steps in the right direction, but manual verification remains brittle. Standardizing on structural validation (e.g. through the Newtype pattern or custom deserializers) ensures that invalid states cannot be parsed into memory at all.
- **Mock Network Reliability**: Hand-rolling TCP protocols and MQTT frame parses in the test mock servers exposes the test harness to concurrency bugs and deadlock conditions. Adopting standard, mature test-mock libraries or converting stream handlers to use tokio/async operations would eliminate the blocking I/O anti-patterns currently present.

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
| `async-trait` | 0.1.86 | Async trait helper (MIT/Apache-2.0) | None |

---

## Appendix: artifacts reviewed

- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\Cargo.toml`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\core\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\printability\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\adapters\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\sdk\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\sdk\src\mocks\rrf.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\sdk\src\mocks\bambu.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\cli\src\main.rs`

# Documentation Deep-Dive — PrintProof3D (Connection Config & Adapter Factory Layer)

**Audit date:** 2026-05-30
**Role:** Technical Writer
**Scope audited:** README, ARCHITECTURE, User Manual, API Reference, FAQ, Changelog, and Contributing Guidelines focusing on the Connection Config layer and Factory.
**Writer mode:** audit+draft
**Auditor posture:** Balanced

---

## TL;DR

The documentation for PrintProof3D's printability validation engine is solid and provides excellent math and WebAssembly sandbox coverage. However, the newly introduced printer connection configuration and adapter factory layer suffer from severe documentation debt. The `PrinterConnectionConfig` data model and the `PrinterAdapterFactory` registry are completely missing from the API Reference and User Manual, and the codebase contains undocumented enums (such as `AuthType::Digest`) and orphaned structures (`SimulatorScenario`). Additionally, the User Manual makes inaccurate claims about the conformance test suite's ability to test simulated failures.

---

## Severity roll-up (documentation)

| Severity | Count |
|---|---|
| Blocker | 0 |
| Critical | 3 |
| Major | 5 |
| Minor | 3 |
| Nit | 0 |

---

## What's working

- **watertight geometry explanations** — `USER_MANUAL.md` Part 2, Section 2.1 provides clear math detailing vertex quantization, manifold shell verification, and overhang slope calculation.
- **WASM Linear Memory documentation** — `ARCHITECTURE.md` Section 2 contains a robust sequence diagram and text explaining the linear memory pointer-length exchange protocol.
- **REST & MCP Config instructions** — `USER_MANUAL.md` Part 2, Section 3 provides exact copy-paste configuration examples for Axum HTTP routes and Claude Desktop JSON integrations.

---

## What couldn't be assessed

No documents were inaccessible. All workspace documentation files (`README.md`, `ARCHITECTURE.md`, `USER_MANUAL.md`, `API_REFERENCE.md`, `FAQ.md`, `CHANGELOG.md`, `CONTRIBUTING.md`) were read in full.

---

## Doc asset inventory

| Asset | Exists? | Status | Finding(s) |
|---|---|---|---|
| README.md | Yes | Adequate | - |
| ARCHITECTURE.md | Yes | Adequate | DOC-008 |
| User manual / guide | Yes | Weak | DOC-003, DOC-006, DOC-007 |
| API reference | Yes | Weak | DOC-001, DOC-002, DOC-004 |
| FAQ | Yes | Adequate | - |
| CHANGELOG | Yes | Adequate | DOC-011 |
| CONTRIBUTING | Yes | Adequate | DOC-007 |
| SECURITY | No | Missing | - |
| LICENSE | No | Missing | - |

---

## Persona walk-through

### First-time user
A first-time user trying to hook up a physical printer or local simulator will get stuck immediately. The `README.md` and `USER_MANUAL.md` only show command-line examples using the `--printer` and `--material` profiles. There are no instructions explaining what a connection configuration JSON file looks like, what keys it requires, or how to write one.

### Returning user
A returning developer looking to programmatically configure a printer connection client will consult `API_REFERENCE.md`. They will fail to find `PrinterConnectionConfig` or `PrinterAdapterFactory` documented anywhere in the manual. They will also find a type mismatch where `PrinterTelemetry.state` is documented as a `String` instead of the typed `PrinterState` enum, leading to compilation issues.

### New team member
A new contributor attempting to add support for a new network printer protocol (e.g. CrealityOS) will implement the `PrinterAdapter` trait, but will not know that they must register their client in `PrinterAdapterFactory::build()`, extend the `ProtocolFamily` enum, or update validation rules in `PrinterConnectionConfig::validate()`, as these steps are omitted from both `CONTRIBUTING.md` and the developer guide.

---

## Findings

> **Finding ID prefix:** `DOC-`
> **Categories:** Accuracy / Completeness / Onboarding / Architecture / API / FAQ / Marketing / Tone / Hygiene

### [DOC-001] — Critical — API — API Reference missing the Connection Config layer

**Evidence**
- File: `API_REFERENCE.md`
- Section: Section 1 (`printproof3d-core` — Shared Models & Schemas)

**Why this matters**
The `PrinterConnectionConfig` struct (and its supporting structures `ConnectionMode`, `AuthType`, and `DispatchPolicy`) is the key domain model for printer integrations, yet it is completely absent from the API Reference. Developers attempting to use this struct in custom integrations are forced to look at the source code (`crates/core/src/connection.rs`) to identify available fields and validation constraints.

**Blast radius**
- `USER_MANUAL.md` also fails to mention the connection configuration.
- Affects external API integrations that rely on these schemas.

**Fix path**
Append the `PrinterConnectionConfig` struct and its helper enums to `API_REFERENCE.md`. (See drafted additions in the "Drafts produced" section).

---

### [DOC-002] — Critical — API — API Reference missing `PrinterAdapterFactory`

**Evidence**
- File: `API_REFERENCE.md`
- Section: Section 3 (`printproof3d-adapters` — Communication Protocols)

**Why this matters**
The `PrinterAdapterFactory` is the central registry for creating concrete adapters (such as `BambuAdapter` or `MoonrakerAdapter`) from profile and connection config objects. Omitting it from the API reference makes it look like adapters must be instantiated manually, hiding the factory pattern design.

**Blast radius**
- Affects developer onboarding and architectural understanding.

**Fix path**
Append the `PrinterAdapterFactory` section to `API_REFERENCE.md` documenting its public `build` method. (See drafted additions in the "Drafts produced" section).

---

### [DOC-003] — Critical — Onboarding — User Manual lacks explanation of Connection Configuration Profiles

**Evidence**
- File: `USER_MANUAL.md`
- Section: Part 1, Section 2 (Profile Structures)

**Why this matters**
While the manual documents the "Printer Profile" and "Material Profile" in detail, it completely neglects the "Connection Configuration Profile." Users cannot learn how to write a configuration file for their printer connection from the manual.

**Blast radius**
- Blocks first-time setup and operator workflows.

**Fix path**
Add a section "2.3 Connection Configuration Profile" to `USER_MANUAL.md` documenting the structure, optional fields, and validation rules. (See drafted additions in the "Drafts produced" section).

---

### [DOC-004] — Major — Accuracy — `PrinterTelemetry::state` Type Mismatch in API Reference

**Evidence**
- File: `API_REFERENCE.md` line 194:
  `pub state: String`
- File: `crates/adapters/src/lib.rs` line 46:
  `pub state: PrinterState`

**Why this matters**
The API reference falsely documents `PrinterTelemetry.state` as a `String`. Integrators writing code against this documentation will experience compilation errors because the compiler expects the `PrinterState` enum. Additionally, the `PrinterState` enum is not documented at all in `API_REFERENCE.md`.

**Blast radius**
- User-facing and developer integrations compiling against the crate.

**Fix path**
Update `API_REFERENCE.md` to document the correct type for `state` and define the `PrinterState` enum.

---

### [DOC-005] — Major — Completeness — JSON Schema generator does not generate `printer_connection_config.schema.json`

**Evidence**
- File: `crates/core/src/lib.rs` line 583 (under `generate_schemas()` test).
- Folder: `schemas/` (only generates `material_profile.schema.json`, `printer_profile.schema.json`, and `validation_report.schema.json`).

**Why this matters**
The project aims to compile JSON schemas automatically to keep other languages (e.g. JavaScript frontend clients) synchronized with the Rust core, but it neglects `PrinterConnectionConfig`. Integration systems in other languages cannot validate connection configs dynamically.

**Blast radius**
- Out-of-repo and frontend validation boundaries.

**Fix path**
Modify the `generate_schemas` test in `crates/core/src/lib.rs` to generate the schema for `PrinterConnectionConfig` and write it to `schemas/printer_connection_config.schema.json`.

---

### [DOC-006] — Major — Accuracy — User Manual makes false claim about Conformance Test Suite capability

**Evidence**
- File: `USER_MANUAL.md` lines 290-291:
  `"...actions execute reliably and return standard AdapterError responses under simulated failures."`
- File: `crates/sdk/src/lib.rs` lines 9-70 (`run_conformance_tests` function).

**Why this matters**
The User Manual claims the conformance test suite automatically verifies adapter behavior under simulated failures and error-state returns. In reality, the test suite only exercises happy-path adapter commands sequentially and fails on the first error. Developers will waste time trying to configure failure modes for the suite that do not exist.

**Blast radius**
- Developer trust in test harnesses.

**Fix path**
Update the User Manual copy to match the true linear nature of `run_conformance_tests`, or extend the conformance runner to inject mock failures.

---

### [DOC-007] — Major — Onboarding — Missing Developer integration instructions for new adapter registration

**Evidence**
- File: `CONTRIBUTING.md`
- File: `USER_MANUAL.md` Part 3, Section 3

**Why this matters**
Adding a new adapter is a common task for contributors. While the docs instruct how to write the adapter, they do not mention that the adapter must be registered in `PrinterAdapterFactory::build()`, the protocol added to `ProtocolFamily` (in `core`), and validation invariants configured in `PrinterConnectionConfig::validate()`. A contributor will write the adapter but find it remains unreachable.

**Blast radius**
- Contributor friction.

**Fix path**
Document the factory registry and validation integration steps in `CONTRIBUTING.md` and `USER_MANUAL.md`.

---

### [DOC-008] — Major — Architecture — Missing Diagram for Connection Config & Adapter Factory instantiations

**Evidence**
- File: `ARCHITECTURE.md`
- Section: Section 1.1 (Architectural Topology Diagram) or Section 3

**Why this matters**
While `ARCHITECTURE.md` has excellent diagrams for WASM guest memory and conformance test servers, it completely lacks a diagram depicting the connection configuration loading and factory resolution flow. A diagram is crucial for understanding how the decoupled crates interact during connection setup.

**Blast radius**
- Architecture understanding.

**Fix path**
Add a Mermaid sequence diagram showing the config loading, validation, and factory mapping sequence. (See drafted additions in the "Drafts produced" section).

---

### [DOC-009] — Minor — Hygiene — Orphaned `SimulatorScenario` enum in connection config module

**Evidence**
- File: `crates/core/src/connection.rs` lines 17-32 (declaration of `SimulatorScenario`).
- Note: It is never referenced in any fields, functions, or tests in the codebase.

**Why this matters**
The presence of `SimulatorScenario` in the connection config module suggests to developers that they can configure specific simulation behaviors (e.g. `BadCredentials` or `StorageFull`) via their connection profiles. Because it is dead code, this causes unnecessary confusion.

**Blast radius**
- Code cleanliness and documentation clarity.

**Fix path**
Either add a `scenario` field to `PrinterConnectionConfig` and wire it to the simulator adapters, or deprecate/remove the enum.

---

### [DOC-010] — Minor — Completeness — Undocumented `AuthType::Digest` auth method

**Evidence**
- File: `crates/core/src/connection.rs` line 40: `Digest` variant in `AuthType`.
- File: `crates/core/src/connection.rs` lines 110-128: `validate()` validator matches only `ApiKey` and `Password` auth types, leaving `Digest` completely unvalidated.

**Why this matters**
`Digest` auth is defined but has no configuration parameters associated with it, nor does the validator check any fields when it is selected. A developer using it will not receive any feedback or validation errors even if their configuration is missing fields.

**Blast radius**
- Validation accuracy.

**Fix path**
Document what fields are expected for `Digest` auth or add specific checks (e.g., validating that both `username` and a credential variable are supplied).

---

### [DOC-011] — Minor — Completeness — Missing Changelog details for the Connection Config and Factory Layer

**Evidence**
- File: `CHANGELOG.md` lines 9-16.

**Why this matters**
The `0.4.0` release notes document the introduction of asynchronous network adapters and tests but completely miss the introduction of the `PrinterConnectionConfig` data model and the `PrinterAdapterFactory` registry. Returning users will miss this architectural change when scanning release notes.

**Blast radius**
- Changelog history correctness.

**Fix path**
Add a line to `CHANGELOG.md` under `[0.4.0]` summarizing the addition of the connection config model and factory.

---

## Drafts produced

The following drafts are provided as inline replacements to resolve the Critical and Major findings:

### Draft 1: API Reference Additions (Resolves DOC-001, DOC-002, DOC-004)

Add the following sections to `API_REFERENCE.md`:

```markdown
### 1.4 `PrinterConnectionConfig` (Struct)
Defines the connection parameters, credentials, endpoints, and pre-flight execution policies required to communicate with a remote printer or simulator.

#### Fields:
* **`name: String`**: A descriptive human-readable label for the target. Must be non-empty.
* **`mode: ConnectionMode`**: Indicates whether to route to a simulation host or physical hardware (`Simulator` or `Physical`).
* **`protocol_family: ProtocolFamily`**: Network/communication protocol enum.
* **`base_url: Option<String>`**: Base URL or IP address. Required for physical network protocol targets.
* **`serial_path: Option<String>`**: Serial port device endpoint path. Required for physical MarlinSerial connections.
* **`serial_baud_rate: Option<u32>`**: Serial connection baud rate.
* **`auth_type: AuthType`**: Authentication mechanism (`None`, `ApiKey`, `Digest`, `Password`).
* **`api_key_env_var: Option<String>`**: Name of the environment variable storing the API key. Required for `AuthType::ApiKey`.
* **`username: Option<String>`**: Username for authentication. Required for `AuthType::Password`.
* **`password_env_var: Option<String>`**: Name of the environment variable storing the password. Required for `AuthType::Password`.
* **`tls_enabled: bool`**: Activates secure socket TLS.
* **`dispatch_policy: DispatchPolicy`**: Pre-flight action permission rules (`DryRunOnly`, `UploadOnly`, `AllowStart`).

#### Key Invariants & Validations:
* `PrinterConnectionConfig::validate(&self) -> Result<(), String>`:
  * Rejects empty target names.
  * For physical mode, checks that `serial_path` is set if protocol is `MarlinSerial`.
  * For physical mode, checks that `base_url` is set if protocol is network-based.
  * For API Key auth, ensures `api_key_env_var` is specified.
  * For Password auth, ensures both `username` and `password_env_var` are specified.

---

### 3.3 `PrinterAdapterFactory` (Struct)
Central registry factory to build dynamic adapter instances from profiles and configurations.

```rust
pub struct PrinterAdapterFactory;

impl PrinterAdapterFactory {
    /// Validates the connection config and returns an initialized dynamic box adapter matching the target protocol.
    pub fn build(
        profile: &PrinterProfile,
        config: &PrinterConnectionConfig,
    ) -> Result<Box<dyn PrinterAdapter>, AdapterError>;
}
```

---

### 3.4 Telemetry Enums
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrinterState {
    Idle,
    Printing,
    Paused,
    Error,
    Unknown,
}
```
```

Also, update `API_REFERENCE.md` line 194 to:
```rust
    pub state: PrinterState,
```

---

### Draft 2: User Manual Additions (Resolves DOC-003, DOC-007)

Add the following section to `USER_MANUAL.md` under Part 1, Section 2:

```markdown
### 2.3 The Connection Configuration Profile (`.json`)
This file details network endpoints, serial configurations, and authentication details required to establish remote control channels:

```json
{
  "name": "OctoPrint Studio",
  "mode": "physical",
  "protocol_family": "octoprint",
  "base_url": "http://192.168.1.50",
  "auth_type": "api_key",
  "api_key_env_var": "OCTOPRINT_API_KEY",
  "tls_enabled": false,
  "dispatch_policy": "allow_start"
}
```

* **Target Modes (`mode`)**:
  * `simulator`: Route commands to test and simulator endpoints.
  * `physical`: Route commands to active machine hardware.
* **Authentication Policies (`auth_type`)**:
  * `none`: Unauthenticated access.
  * `api_key`: Relies on environment variable specified in `api_key_env_var`.
  * `password`: Relies on `username` and password environment variable specified in `password_env_var`.
* **Dispatch Policies (`dispatch_policy`)**:
  * `dry_run_only`: Restricts execution.
  * `upload_only`: Restricts actions to file upload; job starting is blocked.
  * `allow_start`: Full control permission.
```

Update `USER_MANUAL.md` Part 3, Section 3 with the following adapter integration step:

```markdown
### 3.1 Registry Integration Flow
To make your custom adapter reachable by the system:
1. Add the protocol name to the `ProtocolFamily` enum in `crates/core/src/lib.rs`.
2. Update `PrinterConnectionConfig::validate()` in `crates/core/src/connection.rs` to configure any protocol-specific validation checks.
3. Import your adapter and add its instantiation branch to the pattern match block in `PrinterAdapterFactory::build()` inside `crates/adapters/src/factory.rs`.
4. Run the conformance tests in a unit test to confirm integration.
```

---

### Draft 3: Architecture Diagram (Resolves DOC-008)

Insert this section into `ARCHITECTURE.md` under Section 1:

```markdown
### 1.3 Connection Lifecycle & Adapter Instantiation Flow
The diagram below shows the runtime sequence where an integration client parses a connection file, validates it, and instantiates the resolved adapter using the factory:

```mermaid
sequenceDiagram
    autonumber
    participant App as Application Client
    participant Core as Core Crate (PrinterConnectionConfig)
    participant Factory as Adapters Crate (PrinterAdapterFactory)
    participant Adapter as Concrete Adapter (e.g. MarlinSerialAdapter)

    App->>Core: Deserializes connection profile JSON
    Note over Core: Maps to PrinterConnectionConfig struct
    App->>Factory: PrinterAdapterFactory::build(profile, config)
    Factory->>Core: config.validate()
    alt Validation fails
        Core-->>Factory: Err(String)
        Factory-->>App: Err(AdapterError::ConnectionFailed)
    else Validation passes
        Core-->>Factory: Ok(())
        Factory->>Factory: Match config.protocol_family
        create participant Adapter
        Factory->>Adapter: ConcreteAdapter::new(profile, config)
        Adapter-->>Factory: Returns Box<dyn PrinterAdapter>
        Factory-->>App: Ok(Box<dyn PrinterAdapter>)
    end
```
```

---

## Patterns and systemic observations

- **Validation Omissions**: Core types created for adapters are treated as "secondary infrastructure" and skipped in `API_REFERENCE.md` and schema generators. All core structs (specifically `PrinterConnectionConfig`) should be included in the automated schema generation list.
- **Copy-Paste Documentation Drift**: Outdated types (like `state: String`) in docs occur when changes are made to source files (switching to `PrinterState` enum) without performing a full search across the markdown files. Establishing a checklist during PR reviews will prevent type drift.

---

## Appendix: docs reviewed

- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\README.md`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\ARCHITECTURE.md`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\USER_MANUAL.md`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\API_REFERENCE.md`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\FAQ.md`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\CHANGELOG.md`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\CONTRIBUTING.md`

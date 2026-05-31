# Engineering Deep-Dive — PrintProof3D (stage-1-connection-config-factory)

**Audit date:** 2026-05-30
**Role:** Principal Engineer
**Scope audited:** Connection config model and adapter factory on branch `stage-1-connection-config-factory`. Files reviewed: `crates/core/src/connection.rs`, `crates/core/src/lib.rs`, `crates/adapters/src/factory.rs`, `crates/adapters/src/lib.rs`, `crates/adapters/src/bambu.rs`, `crates/adapters/src/moonraker.rs`, `crates/adapters/src/octoprint.rs`, `crates/adapters/src/prusalink.rs`, `crates/adapters/src/rrf.rs`, `crates/adapters/src/serial.rs`, `crates/sdk/src/lib.rs`, `crates/sdk/src/mocks/bambu.rs`, `crates/sdk/src/mocks/rrf.rs`, `crates/sdk/src/mocks/mod.rs`.
**Auditor posture:** Balanced / Professional

---

## TL;DR

The connection config model and adapter factory module compile cleanly, but they suffer from significant design omissions, extensibility bottlenecks, and security trade-offs that must be addressed before shipping to production. 

Most notably, the `SimulatorScenario` enum is fully defined in the codebase but was entirely omitted from the `PrinterConnectionConfig` struct, making it impossible to configure or test specific simulator scenarios. The adapter factory uses a hardcoded match statement that violates the Open-Closed Principle, blocking external plugins from registering custom protocol drivers. Secret resolution is tied rigidly to global process environment variables, which prevents the application from scaling in multi-tenant environments (e.g., print farms) and introduces thread-safety concerns. Local TLS certificate validation is missing entirely, forcing a choice between unusable HTTPS (due to self-signed certificates) and plaintext connections. Finally, the test mock servers leak OS threads and sockets because they use blocking read operations inside their main loops.

There are no Blockers, but there are 5 Major findings, 3 Minor findings, and 0 Nits.

## Severity roll-up (engineering)

| Severity | Count |
|---|---|
| Blocker | 0 |
| Critical | 0 |
| Major | 5 |
| Minor | 3 |
| Nit | 0 |

## What's working

- **Basic Config Validation Invariants**: `PrinterConnectionConfig::validate()` correctly performs basic structural checks, such as ensuring a name is provided, `serial_path` is present for serial devices, and `base_url` is present for network devices.
- **Factory Registry Mapping**: `PrinterAdapterFactory::build()` successfully routes supported `ProtocolFamily` variants to their respective adapter structures.
- **Clean Schema Generation**: Schemars integration and serialization tests correctly output JSON schema definitions for the core printer and material profiles.
- **WASM Plugin Execution**: The `printproof3d-plugins` crate correctly compiles, loads, and executes WASM plugins with isolated memory exchanges.
- **API and CORS Middleware**: The Axum REST server correctly implements authentication middleware and enforces local origin policies for CORS.

## What couldn't be assessed

- **Production Adapter Behavior**: Since all 6 printer adapters (`bambu`, `moonraker`, `octoprint`, `prusalink`, `rrf`, `serial`) are currently skeleton implementations, we could only audit their interfaces and factory routing. Live network behavior could not be evaluated.
- **Hardware Integration**: Actual serial and network protocol parsing against physical 3D printer hardware was not tested.

---

## Findings

### [ENG-001] — Major — Correctness — Dead Code and Omission of `SimulatorScenario` in Configuration Struct

**Evidence**
- `crates/core/src/connection.rs:17-32` vs `crates/core/src/connection.rs:53-80`
The enum `SimulatorScenario` is fully defined with standard simulation states (e.g., `Idle`, `AlreadyPrinting`, `Paused`, `BadCredentials`, `TimeoutOrSlowResponse`):
```rust
pub enum SimulatorScenario {
    Idle,
    AlreadyPrinting,
    Paused,
    ...
}
```
However, `SimulatorScenario` is completely missing as a field in `PrinterConnectionConfig`:
```rust
pub struct PrinterConnectionConfig {
    pub name: String,
    pub mode: ConnectionMode,
    pub protocol_family: ProtocolFamily,
    // ... no simulator_scenario field
}
```
As a result, the compiler flags the enum as unused dead code, and there is no way for a user or developer to specify or test a simulation scenario when configuring a connection in `Simulator` mode.

**Why this matters**
A primary use-case of simulator mode is dry-running print jobs and QA'ing printer state transitions under controlled error states (e.g., storage full, malformed telemetry). Without a way to specify the scenario in the configuration, the simulator will always fall back to a hardcoded default behavior (or fail), making simulated connections functionally useless for testing state transitions.

**Blast radius**
- `PrinterConnectionConfig` structure.
- Simulator adapters and testing framework.
- Validation API and frontend configuration panels.

**Fix path**
Add an optional `simulator_scenario` field to `PrinterConnectionConfig`:
```rust
pub simulator_scenario: Option<SimulatorScenario>,
```
Update `validate()` to ensure that if `mode == ConnectionMode::Simulator`, a default scenario is assumed or verified.

---

### [ENG-002] — Major — Architecture — Hardcoded Adapter Factory Registry Restricts Extensibility (OCP Violation)

**Evidence**
- `crates/adapters/src/factory.rs:20-48`
The factory instantiates adapters using a static match statement:
```rust
        match config.protocol_family {
            ProtocolFamily::BambuMqtt => {
                Ok(Box::new(BambuAdapter::new(profile.clone(), config.clone())))
            }
            ProtocolFamily::Klipper => Ok(Box::new(MoonrakerAdapter::new(
                profile.clone(),
                config.clone(),
            ))),
            // ...
```
This hardcoded registry violates the Open-Closed Principle (OCP). If a developer or a plugin (like `example-plugin`) wants to register a new driver for unsupported families (such as `ElegooSdcp`, `CrealityOs`, `AnycubicLan`, etc., which are defined in `ProtocolFamily`), they cannot do so without modifying the core `adapters` crate.

**Why this matters**
Tight coupling between the factory and individual adapter crates makes it impossible to package adapters in independent modules or let users supply their own connection drivers. It also requires the `adapters` crate to carry dependencies for all supported network protocols, increasing compilation times and binary bloat.

**Blast radius**
- `PrinterAdapterFactory` registry.
- Core adapter loading pipeline.
- Custom/plugin printer driver implementations.

**Fix path**
Refactor the factory to use a dynamic registry pattern. Introduce a registry interface (such as a thread-safe map of constructor functions) allowing adapters to register themselves at runtime:
```rust
HashMap<ProtocolFamily, Box<dyn Fn(PrinterProfile, PrinterConnectionConfig) -> Box<dyn PrinterAdapter> + Send + Sync>>
```

---

### [ENG-003] — Major — Security — Rigid Environment Variable Resolution for Secrets Prevents Multi-tenancy

**Evidence**
- `crates/core/src/connection.rs:70-75` and `validate()` lines 110-128
The connection configuration stores environment variable *names* to resolve secrets:
```rust
    pub api_key_env_var: Option<String>,
    pub password_env_var: Option<String>,
```
This architecture assumes secrets must be resolved by looking up process-level environment variables (e.g., `std::env::var("OCTOPRINT_API_KEY")`).

**Why this matters**
In a multi-tenant environment (such as a printer dashboard managing multiple printers of the same model with different API keys), this forces administrators to define a distinct environment variable for every single printer (e.g., `KEY_1`, `KEY_2`), which is highly rigid. Furthermore, mutating process-level environment variables dynamically at runtime in a multi-threaded web server is unsafe in Rust (causing races and potential undefined behavior). Finally, `validate()` only checks if the variable name field is present, not if the environment variable itself is actually set.

**Blast radius**
- Authentication resolution.
- Multi-printer network configurations.
- Thread-safe runtime environment.

**Fix path**
Introduce a `CredentialSource` enum that can represent an environment variable, an encrypted configuration database reference, or a direct transient in-memory secret string. Provide a credential resolver trait or callback context to the factory at construction time.

---

### [ENG-004] — Major — Security — Lack of TLS Certificate Pinning and Verification Settings

**Evidence**
- `crates/core/src/connection.rs:76-77`
The model exposes a simple boolean `tls_enabled: bool`, but lacks options for certificate validation:
```rust
    /// Secure socket TLS state.
    pub tls_enabled: bool,
```

**Why this matters**
Local network printer controllers (OctoPrint, Moonraker, PrusaLink) typically use self-signed TLS certificates. Without a way to configure the adapter to allow self-signed certificates, load custom CA roots, or verify a certificate's SHA-256 fingerprint, enabling TLS will cause standard HTTP/WebSocket connections to fail. Consequently, operators are forced to turn TLS off entirely (`tls_enabled: false`), exposing sensitive credentials and API keys in plaintext over the local network.

**Blast radius**
- Connection adapters.
- Local network security posture.

**Fix path**
Add verification settings to `PrinterConnectionConfig`:
```rust
pub tls_allow_invalid_certs: bool,
pub tls_ca_cert_pem: Option<String>,
pub tls_pinned_fingerprint: Option<String>,
```
Ensure adapters pass these options to the underlying network client builders (e.g., reqwest, mqtt, or serial).

---

### [ENG-005] — Major — Performance — Blocking Socket Reads in SDK Mock Servers Leak Threads and Sockets

**Evidence**
- `crates/sdk/src/mocks/bambu.rs:27, 117` and `crates/sdk/src/mocks/rrf.rs:26`
Mock server implementations (such as `BambuFtpMock` and `RrfMockServer`) spawn threads that run blocking read operations:
```rust
                    while let Ok(n) = stream.read(&mut buffer) {
                        if n == 0 {
                            break;
                        }
                        ...
```
If a client connects but does not send data, the thread blocks indefinitely in `stream.read()`. If `stop()` is called (which sets the `running` atomic boolean to false), the loop is unable to check the atomic flag because it is blocked waiting on the socket syscall.

**Why this matters**
This causes background OS thread and file descriptor leaks during test suite execution. Over multiple test runs, this can lead to system resource exhaustion and flaky tests, especially in CI environments where tests are run in parallel.

**Blast radius**
- SDK testing harness.
- Local CI resources.

**Fix path**
Set a read timeout on the accepted sockets:
```rust
stream.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;
```
Alternatively, rewrite the mock servers using async Rust (`tokio::net::TcpListener`) and `tokio::select!` to handle shutdowns cleanly.

---

### [ENG-006] — Minor — Correctness — Incomplete Validation of Protocol/Mode Combinations in `validate()`

**Evidence**
- `crates/core/src/connection.rs:93-107` vs `crates/adapters/src/factory.rs:43-46`
The config validation function contains logic that skips network checking if the protocol family is `Unknown`:
```rust
                let need_base_url = !matches!(self.protocol_family, ProtocolFamily::Unknown);
                if need_base_url && self.base_url.is_none() { ... }
```
If `mode == ConnectionMode::Physical` and `protocol_family == ProtocolFamily::Unknown`, validation succeeds even if `base_url` and `serial_path` are both `None`. However, passing this validated config to `PrinterAdapterFactory::build()` will immediately trigger an instantiation error:
```rust
            _ => Err(AdapterError::ConnectionFailed(format!(
                "Unsupported protocol family: {:?}",
                config.protocol_family
            ))),
```

**Why this matters**
This is a contract mismatch between the data model validator and the factory. A configuration that successfully passes `validate()` will fail immediately at runtime in the factory, causing confusing error paths.

**Blast radius**
- `PrinterConnectionConfig::validate()` logic.

**Fix path**
Ensure `validate()` rejects `ProtocolFamily::Unknown` under physical mode, or explicitly checks that some connection target endpoint is configured.

---

### [ENG-007] — Minor — Architecture — Missing `#[serde(default)]` on Configuration Fields

**Evidence**
- `crates/core/src/connection.rs:55-80`
Fields such as `tls_enabled`, `dispatch_policy`, `auth_type`, `username`, and option fields are declared without default fallback markers.

**Why this matters**
If a JSON connection configuration saved to disk lacks one of these optional fields (e.g., an older config format omitting `tls_enabled` or `dispatch_policy`), parsing will fail completely. This breaks backward compatibility and prevents the system from assuming safe defaults for omitted settings.

**Blast radius**
- Configuration deserialization.

**Fix path**
Add `#[serde(default)]` to non-critical fields so they default to sensible values (e.g., `tls_enabled` to `false`, `auth_type` to `AuthType::None`, `dispatch_policy` to `DispatchPolicy::DryRunOnly`).

---

### [ENG-008] — Minor — Correctness — Test Client Telemetry Data Validation is Hardcoded

**Evidence**
- `crates/sdk/src/lib.rs:135-143` and `248-256`
The conformance test clients (`RrfTestClient` and `BambuTestClient`) check that the server returns a valid response but construct the telemetry struct using hardcoded values:
```rust
                        return Ok(PrinterTelemetry {
                            state,
                            tool_temp: 210.0,
                            tool_target: 210.0,
                            bed_temp: 60.0,
                            bed_target: 60.0,
                            progress: 50.0,
                            current_file: None,
                        });
```

**Why this matters**
This bypasses data provenance verification in the tests. The conformance suite verifies that a telemetry response is returned, but does not confirm that the adapter is actually reading the *correct runtime values* parsed from the mock server payload. 

**Blast radius**
- SDK conformance tests.

**Fix path**
Update the test clients to parse the JSON and MQTT payloads from the mock server and populate the telemetry fields with the parsed variables.

---

## Patterns and systemic observations

- **Validation / Factory Discrepancies**: Checking config properties synchronously without verifying environment variables or validating the protocol family limits the utility of `validate()`. A unified validation approach is needed.
- **Resource Management in Tests**: The reliance on blocking threads and sockets inside tests indicates a need for a modern asynchronous testing framework.
- **Dynamic Extensibility**: The hardcoded adapter registry structure should be migrated to a dynamic factory pattern early in the design cycle before more adapters are introduced.

## Dependency snapshot

Versions retrieved from `Cargo.lock`. No critical CVE concerns were found for these versions.

| Dependency | Version | Concern |
|---|---|---|
| `axum` | 0.7.9 | Axum REST web framework. |
| `serde` | 1.0.228 | Serialization and deserialization framework. |
| `schemars` | 0.8.22 | JSON Schema generator. |
| `wasmi` | 0.31.2 | WASM interpreter for plugins. |
| `tokio` | 1.35.0 (dev) | Runtime driver for SDK tests. |

## Appendix: artifacts reviewed

- `crates/core/src/connection.rs` (Config model structure and validation rules)
- `crates/core/src/lib.rs` (Core type definitions)
- `crates/adapters/src/factory.rs` (Adapter factory construction and routing)
- `crates/adapters/src/lib.rs` (Adapter traits and state mappings)
- `crates/adapters/src/*.rs` (Individual adapter skeleton structures)
- `crates/sdk/src/lib.rs` (SDK conformance tests and client mock implementations)
- `crates/sdk/src/mocks/*.rs` (Mock servers for Bambu MQTT/FTP and RepRapFirmware)
- `Cargo.toml` and `Cargo.lock` (Workspace dependency versions)

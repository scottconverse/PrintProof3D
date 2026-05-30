# PrintProof3D Discussion Seeds

This document contains ideas, design debates, and roadmap proposals intended to start discussions on issues and community forums.

---

## 💬 Topic 1: Dynamic WASM Plugins vs. Scripting Languages

Our Stage 4 implementation uses a sandboxed WebAssembly runtime (`wasmi`) to run validation plugins. 

### Seed Questions:
1. **Developer Friction**: How high is the entry barrier for writing plugins? Should we provide an AssemblyScript or JavaScript/TypeScript SDK for writing plugins in addition to Rust?
2. **Execution Overhead**: The `wasmi` interpreter is pure Rust and requires no native compilation. Is its execution speed sufficient for large ValidationReports containing millions of G-code coordinate lines, or should we eventually support optional JIT engines (like Wasmtime) under a cargo feature?
3. **Data Exchange Formats**: Currently, we serialize reports to JSON strings. Would moving to FlatBuffers or Protocol Buffers over WASM linear memory provide enough speedup to justify the schema complexity?

---

## 💬 Topic 2: Extending the Adapter Conformance Suite

Our SDK includes a conformance harness `run_conformance_tests` that validates connection states and control commands.

### Seed Questions:
1. **Failure Injection**: How should we test adapter resilience under unstable network connections (e.g., packet drops, serial timeouts)? Should the compliance harness accept a fault-config parameter to inject delays?
2. **Firmware Specifics**: Different firmwares (Marlin vs. RepRap vs. Klipper) have different behaviors for emergency stops and command buffering. Should the compliance suite expose specialized tests based on the adapter's declared `FirmwareFlavor`?

---

## 💬 Topic 3: Real-Time Telemetry Auditing & Safety Constraints

Currently, G-code check runs statically. We could extend the adapters/plugins system to run rules *during* print execution (e.g. auditing current temperature drift telemetry against safety envelopes).

### Seed Questions:
1. **Safety Envelopes**: Should plugins be allowed to trigger an abort command if they detect telemetry anomalies (e.g. thermal runaway patterns) via the adapter?
2. **Resource Consumption**: How can we run real-time checks in a background thread without starving the main G-code stream loop, especially on low-power devices like a Raspberry Pi?

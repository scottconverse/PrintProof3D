# Security Policy

This document describes the security policies and vulnerability disclosure procedures for **PrintProof3D**.

## Supported Versions

Only the latest release (including release candidates) is supported with security updates.

| Version | Supported |
| --- | --- |
| `0.5.0-rc1` | :white_check_mark: Yes |
| < `0.5.0-rc1` | :x: No |

## Reporting a Vulnerability

Confidential security reports can be sent directly to `sconverse@gmail.com`. Please **do not** open public issues or pull requests for security vulnerabilities.

### Vulnerability Report Checklist
Please include the following in your report:
1. **Description**: Detail the vulnerability, affected components, and potential impact.
2. **Steps to Reproduce**: Provide clear, step-by-step instructions or a minimal proof-of-concept.
3. **Environment**: Version, compilation targets, and platform (e.g., Windows, Linux).
4. **Assessment**: Initial severity assessment (Low, Medium, High, Critical).

We aim to acknowledge receipt of reports within 3 business days and provide a coordinated fix schedule based on severity.

## Scope and Security Architecture

### Wasmi Sandboxing
All dynamic third-party validation plugins run inside a sandboxed WebAssembly execution environment powered by `wasmi`. The runtime enforces:
- **Linear Memory Ceiling**: Restricts linear memory growth to `16 MB` via custom resource limiters to prevent resource exhaustion or out-of-memory (OOM) host crashes.
- **Instruction Fuel Limits**: Meters execution to a maximum of `50,000,000` instructions per call to prevent infinite loops, hangs, or denial of service.

### Axum REST API Security
- **Authentication**: All endpoints that accept model/G-code validation data or perform profile introspection require bearer token authentication.
- **Bearer Tokens**: Start empty by default. Ephemeral startup tokens are dynamically generated using PID and time entropy if `PRINTPROOF3D_API_TOKEN` is not set in the environment. Tokens are never stored by the UI unless explicitly checked by the user.
- **CORS Configuration**: Restricts access to localhost and loopback domains (`127.0.0.1`, `localhost`).
- **Static Assets Compilation**: All static dashboard components and loaders are compiled directly into the binary using `include_str!` or `include_bytes!`, mitigating path traversal exploits.

## Known Limitations and Disclaimers

- **Simulator and Verification Invariants**: PrintProof3D is a simulator and static validation utility. It performs pre-flight check parsing of coordinates, profiles, and path parameters, but does not certify physical outcomes, machine behavior, or completed prints.
- **Physical Machine Trust Boundary**: We assume the local machine running the REST API is safe. Process-level isolation or loopback interception is not defended against if the host operating system is already compromised.

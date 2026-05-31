# UI/UX & DX Deep-Dive — PrintProof3D (Connection Configs, CLI, and Web Surfaces)

**Audit date:** 2026-05-30  
**Role:** Senior UI/UX Designer  
**Scope audited:** The new PrinterConnectionConfig model, adapter factory registry, conformance SDK suite, and web surfaces (index, manual, architecture spec, api reference) on the `stage-1-connection-config-factory` branch.  
**Auditor posture:** Balanced  

---

## TL;DR

PrintProof3D’s connection config factory slice compiles cleanly, has thorough configuration model validation logic, and introduces an automated conformance test harness for adapters. However, the developer experience (DX) and user interfaces contain several major structural gaps:
1. **Zero UI/Interface Exposure:** The printer connection configurations and adapter factory registry are completely unexposed to the CLI, Axum REST API, and MCP server. Developers and operators cannot inspect, test, or manage printer connections through any interface without writing custom Rust code.
2. **Critical Responsive Documentation Layouts:** The user manual, architecture specification, and API reference pages contain absolutely no `@media` query overrides. On viewport widths less than 1024px (mobile/tablet), the sticky sidebar remains locked at 300px, causing severe content squashing, text truncation, and massive horizontal overflow that renders the documentation unreadable.
3. **Typography Code Font Bug:** The global font stack for code blocks, preformatted tags, and interactive JSON/Terminal viewers incorrectly specifies `Space Grotesk` (a proportional sans-serif display font) as the primary font before `monospace`, leading to misaligned columns, broken spacing, and illegible code.
4. **Ad Hoc Web Accessibility (a11y):** The interactive tab switcher on the homepage lacks WAI-ARIA roles, attributes (`role="tab"`, `aria-selected`), and keyboard arrow navigation. Keyboard outlines are entirely browser-default, which have poor contrast on the custom dark background.
5. **Contract and Validation Loopholes:** The data model allows invalid configurations at deserialization time, the factory doesn't validate printer profiles, and mismatches between connection profiles and adapter configurations pass validation but trigger misleading runtime failures.

Addressing these issues will elevate PrintProof3D from a collection of isolated backend libraries into an integrated, accessible, and production-ready product.

---

## Severity roll-up (UX/DX)

| Severity | Count |
|---|---|
| Blocker | 0 |
| Critical | 1 |
| Major | 7 |
| Minor | 3 |
| Nit | 2 |

---

## What's working

- **Robust Config Validation Logic:** `PrinterConnectionConfig::validate()` correctly checks and errors on missing configurations (e.g. missing `serial_path` for physical serial, missing `base_url` for physical network, and missing credential parameters for specific auth types). Actionable error descriptions are tested.
- **Factory Registry Mapping:** `PrinterAdapterFactory::build()` successfully validates connection profiles and returns a correct dynamic Box adapter mapping to Klipper, OctoPrint, BambuMqtt, PrusaLink, RepRapFirmware, and MarlinSerial.
- **Visual Palette and Identity:** The dark glassmorphic design theme (`#090b10`) on the web surfaces is modern, clean, and has high contrast text (`#e2e8f0`) that exceeds WCAG 2.1 AA requirements.
- **Unified Live Architecture Spec:** The inclusion of live Mermaid diagrams on `index.html` and `architecture.html` is an excellent DX design choice that maps crate boundaries and WebAssembly memory sharing protocols clearly.

---

## What couldn't be assessed

- **Active Network Telemetry & Real Handshakes:** The network connection adapters (Moonraker, OctoPrint, PrusaLink, RepRapFirmware, Bambu) are implemented as stubs returning "Not implemented" errors on connection or command execution. Consequently, the actual network latency handling, reconnect states, and active connection errors could not be observed in a running environment.
- **CLI/REST runtime connection commands:** Since there are no CLI flags or REST endpoints implemented for printer connection management, the command-line visual layouts, interactive prompt flows, and JSON-RPC outputs for connection states could only be analyzed from the static code.

---

## First impressions

**Web Landing Page:** Arriving at the landing page, the value proposition is immediately clear — "Verify Geometries & Paths Before You Press Print." The interactive terminal mockup showing an STL overhang warning report is clean and informative. However, upon resizing the screen to a tablet width, the navigation links immediately bunch together, and on mobile, the entire documentation area breaks into an unusable dual-column view.

**Developer SDK / Configs:** Opening `crates/core/src/connection.rs`, a developer is greeted with clear, structured enums and structs. However, the developer is immediately forced to model connections as a flat struct with multiple optional fields that are conditionally required (e.g., providing a `serial_path` only when `ProtocolFamily` is `MarlinSerial`), which feels unidiomatic in Rust.

---

## Journey walkthroughs

### Journey: Operator configures and tests a new Klipper connection
1. The operator reads `user_manual.html#compliance-tests` and finds the data fields required for a connection.
2. They create a configuration file matching `PrinterConnectionConfig`. They must specify a name, protocol, and base URL.
3. **Dead End:** The operator runs the CLI tool (`printproof3d --help`) to test their connection config or see if the printer is reachable. They discover there is no command to test, run, or list connection profiles.
4. They open the REST API docs and find that `/profiles/printers` and `/validate/model` are available, but there are no connection management endpoints.
5. **Outcome:** The operator cannot use or test their connection config unless they write a custom Rust program and compile it.

### Journey: Developer views User Manual on a tablet or mobile phone
1. A developer on a tablet or phone opens `user_manual.html` to reference the WebAssembly memory allocation macros.
2. **Breakage:** The layout remains locked in a 300px / 1fr grid. The sidebar navigation takes up nearly the entire screen, and the actual documentation content is squashed into a narrow 150px vertical strip, causing massive text wrapping and clipping.
3. The developer is forced to zoom out or switch to a desktop computer to read the instructions.

---

## Findings

### [UX-001] — Major — Journey / IA — Connection configs and adapter factory are unexposed to CLI, REST, and MCP

**Evidence**
- `crates/cli/src/main.rs`: The CLI only exposes `Mcp`, `ValidateModel`, and `ValidateGcode` subcommands.
- `crates/cli/src/mcp.rs`: The MCP tools only include `validate_model_printability`, `validate_gcode`, `list_printer_profiles`, and `explain_validation_report`.
- `crates/rest/src/main.rs`: The REST router only includes `/`, `/profiles/printers`, `/validate/model`, and `/validate/gcode`.

**Why this matters**
A core feature of this branch is the connection configuration model and the printer adapter factory. However, because they are completely unexposed to the primary user interfaces (CLI, REST, MCP), operators cannot use them without writing custom Rust code. A user cannot run `printproof3d test-connection` or make a `POST /connection/test` request to verify their printer settings. This creates a major gap in the developer and operator journey.

**Blast radius**
- Affects any developers or DevOps engineers trying to integrate PrintProof3D into print farm management software or local CI pipelines.

**Fix path**
- Add a `test-connection` subcommand to the CLI that accepts a path to a connection config file and a printer profile, builds the adapter using `PrinterAdapterFactory::build()`, and runs a status check or connection handshake.
- Add `GET /connections` and `POST /connections/test` endpoints to the REST API.
- Register a `test_printer_connection` tool in the MCP server.

---

### [UX-002] — Minor — Typography — Monospace font fallback typography bug in web code blocks

**Evidence**
- `index.html` (lines 38, 371, 401-406): `code` and `.code-pane` styles declare `font-family: 'Space Grotesk', monospace;`.
- `user_manual.html` (line 19, 222), `architecture.html` (line 35, 234), `api_reference.html` (line 19, 202) also use `font-family: 'Space Grotesk', monospace;` for `code` and `pre` elements.

**Why this matters**
`Space Grotesk` is a sans-serif display font, not a monospace font. By placing it before `monospace` in the font stack, browsers will render G-code instructions, terminal commands, JSON responses, and Rust source snippets in a proportional sans-serif font. This destroys the horizontal alignment of G-code coordinates, breaks the layout structure of JSON objects, and degrades the visual readability of all technical code blocks.

**Blast radius**
- Every code element and terminal block across all four HTML documentation pages.

**Fix path**
Update the font-family stack for `code`, `pre`, and `.code-pane` to a proper monospace stack:
```css
code, pre, .code-pane, .json-code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
}
```

---

### [UX-003] — Critical — Responsive — Documentation pages do not support viewports < 1024px

**Evidence**
- `user_manual.html` (lines 80-83), `architecture.html` (lines 96-99), `api_reference.html` (lines 80-83): 
```css
.doc-layout {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 3rem;
}
```
No `@media` query blocks exist in any of these three files to override this layout.

**Why this matters**
On a mobile device (320px) or tablet (768px), the sticky sidebar is locked at 300px wide, leaving zero or very little space for the main documentation content. This causes massive horizontal overflow, clipping of text, and makes the documentation completely unreadable on mobile screens. Documentation should stack into a single column on smaller viewports.

**Blast radius**
- `user_manual.html`, `architecture.html`, and `api_reference.html` layouts.

**Fix path**
Add a responsive media query at `1024px` to collapse the grid layout into a single column, stack the sidebar above or below the content (or make it a toggleable navigation drawer), and adjust padding:
```css
@media (max-width: 1024px) {
  .doc-layout {
    grid-template-columns: 1fr;
    gap: 1.5rem;
  }
  .sidebar {
    position: static;
    height: auto;
    margin-bottom: 2rem;
  }
}
```

---

### [UX-004] — Major — Pattern / DX — Flat data models in PrinterConnectionConfig allow invalid configurations at compile/deserialization time

**Evidence**
- `crates/core/src/connection.rs`:
```rust
pub struct PrinterConnectionConfig {
    pub name: String,
    pub mode: ConnectionMode,
    pub protocol_family: ProtocolFamily,
    pub base_url: Option<String>,
    pub serial_path: Option<String>,
    pub serial_baud_rate: Option<u32>,
    pub auth_type: AuthType,
    pub api_key_env_var: Option<String>,
    pub username: Option<String>,
    pub password_env_var: Option<String>,
    pub tls_enabled: bool,
    pub dispatch_policy: DispatchPolicy,
}
```

**Why this matters**
By modeling optional fields in a single flat structure, Rust's type-safety guarantees are bypassed. The design allows impossible/conflicting configurations to be constructed in memory or deserialized from JSON (e.g. setting both serial paths and network URLs, or choosing serial mode with HTTP API keys). This shifts the verification burden to runtime `validate()` checks rather than enforcing correctness at compile/deserialization time.

**Blast radius**
- Configuration loading, internal APIs, and serialization schemas in `crates/core`.

**Fix path**
Refactor the connection config to use structured enum variants (algebraic data types) to separate serial and network parameters:
```rust
pub struct PrinterConnectionConfig {
    pub name: String,
    pub mode: ConnectionMode,
    pub transport: PrinterTransport,
    pub dispatch_policy: DispatchPolicy,
}

pub enum PrinterTransport {
    Network {
        protocol: NetworkProtocol,
        base_url: String,
        auth: NetworkAuth,
        tls_enabled: bool,
    },
    Serial {
        serial_path: String,
        baud_rate: u32,
    },
}
```

---

### [UX-005] — Major — Pattern / DX — Closed static factory registry restricts extensibility (OCP Violation)

**Evidence**
- `crates/adapters/src/factory.rs:20-48`:
Instantiates adapters via a static match pattern on `ProtocolFamily`.

**Why this matters**
This violates the Open-Closed Principle. Developers wanting to add custom adapters for new protocols (e.g. CrealityOS or custom farm management protocols) cannot do so dynamically. They must modify the core `adapters` crate and the `ProtocolFamily` enum, which limits modularity and blocks third-party plugin integrations.

**Blast radius**
- Crate design coupling and third-party driver ecosystem.

**Fix path**
Introduce a dynamic registration pattern where adapter drivers register constructor closures under a string or enum key in a thread-safe registry map at startup.

---

### [UX-006] — Major — Pattern / DX — Conformance test suite runs against custom stubs rather than production adapters

**Evidence**
- `crates/sdk/src/lib.rs` (lines 9-70, 180-188, 305-318):
The tests pass private local structs (`RrfTestClient` and `BambuTestClient`) to the conformance suite rather than the actual `RrfAdapter` and `BambuAdapter` defined in `crates/adapters`.

**Why this matters**
This creates a false signal of test success. The conformance suite asserts that the connection and command logic works, but it runs against separate testing stubs. The production adapters remain untested skeletons that return "Not implemented" errors, meaning bugs in production code are completely masked.

**Blast radius**
- Entire QA validation feedback loop.

**Fix path**
Implement the actual production adapters and update the integration test suite to run the conformance assertions against the production adapter structs communicating with the mock servers.

---

### [UX-007] — Minor — DX / Hygiene — SDK conformance suite clutters current working directory with test file

**Evidence**
- `crates/sdk/src/lib.rs` lines 40-50:
Writes `test_upload_conformance.gcode` to the current working directory during execution.

**Why this matters**
Running tests should never mutate the user's current working directory or pollute git status. This causes problems in parallel test runners and crashes in read-only environments (like sandboxed CI/CD containers).

**Blast radius**
- Developer local workspaces and parallel test suites.

**Fix path**
Use a temporary directory helper via the `tempfile` crate to write and automatically clean up the file.

---

### [UX-008] — Major — Accessibility — Homepage tab switcher lacks WAI-ARIA roles and attributes

**Evidence**
- `index.html` lines 434-438:
The tab list container and button elements are plain HTML `div` and `button` tags with only visual active classes.

**Why this matters**
Assistive technologies cannot determine that these controls represent a tabbed interface or which tab is selected. This makes it impossible for screen reader users to navigate the interactive terminal preview, violating accessibility guidelines.

**Blast radius**
- Homepage landing experience.

**Fix path**
Add `role="tablist"` to the container, and `role="tab"`, `aria-selected="true/false"`, `aria-controls="pane-id"`, and `tabindex` attributes to the buttons.

---

### [UX-009] — Major — Accessibility — Missing custom focus outlines and poor focus indicator contrast

**Evidence**
- `index.html`, `user_manual.html`, `architecture.html`, `api_reference.html`:
No custom focus indicator outlines are defined in any stylesheets.

**Why this matters**
Visual keyboard-only users rely on visible focus indicators to know where they are. Browser default outlines are often thin, blue glows that blend into the dark-blue gradient backgrounds and glowing cards, failing accessibility contrast guidelines on dark-themed sites.

**Blast radius**
- Global web surfaces.

**Fix path**
Implement a prominent custom focus outline in the global CSS:
```css
a:focus-visible, button:focus-visible {
  outline: 2px solid var(--secondary);
  outline-offset: 4px;
}
```

---

### [UX-010] — Nit — DX — Missing JSON Schema generation for connection configurations

**Evidence**
- `crates/core/src/lib.rs` (under the `generate_schemas()` test):
`PrinterConnectionConfig` is decorated with `#[derive(JsonSchema)]` but is omitted from the files exported to `schemas/`.

**Why this matters**
Integrators and developers writing JSON configs lack auto-complete validation support in IDEs (like VS Code) and client-side form validation mechanisms.

**Blast radius**
- Workspace schemas export.

**Fix path**
Add a schema output step for `PrinterConnectionConfig` to the `generate_schemas` test.

---

### [UX-011] — Nit — DX / Validation — Baud rate validation boundary gap

**Evidence**
- `crates/core/src/connection.rs`:
`serial_baud_rate` lacks checking, allowing rates like `0` or negative values to pass validation.

**Why this matters**
Allowing incorrect boundary speeds passes config validation but will trigger runtime failures or library panics during serial initialization.

**Blast radius**
- Serial connection validator.

**Fix path**
Add a sanity check in `validate()` ensuring the baud rate is a positive, standard speed if MarlinSerial is selected.

---

### [UX-012] — Minor — DX / API — MCP report explanation tool expects raw escaped string instead of JSON object

**Evidence**
- `crates/cli/src/mcp.rs`:
`explain_validation_report` expects `report_json` as a raw escaped string.

**Why this matters**
Forces MCP clients (like Claude Desktop) to perform complex escaping of multi-line JSON structures into a single line string, causing parsing bugs and tool failures.

**Blast radius**
- MCP Server interface tools.

**Fix path**
Update the parameter schema to accept a structured `serde_json::Value` directly.

---

### [UX-013] — Major — DX / API — Inconsistent protocol validation between core validator and factory

**Evidence**
- `crates/core/src/connection.rs` vs `crates/adapters/src/factory.rs`:
Selecting `ProtocolFamily::ElegooSdcp` passes core validation but returns a runtime `ConnectionFailed` error from the factory.

**Why this matters**
Mismatches between the validation rules and runtime registry capabilities cause confusing error reporting. The operator is told the config is valid, but the factory fails during initialization.

**Blast radius**
- Adapter registry instantiation.

**Fix path**
Verify protocol compatibility inside `validate()` or return an explicit `Unsupported` error from the factory.

---

## Patterns and systemic observations

- **Decoupling vs. Exposure (IA):** While the crates are mathematically clean and isolated, there is a systemic lack of exposure of connection configs and adapters. If a feature is developed but not hooked into the CLI or REST interfaces, it remains a "dead" feature for non-developer operators.
- **WAI-ARIA Completeness:** Custom interactive widgets (like tab bars and theme togglers) must have standard accessibility roles. Relying only on visual classes (`.active`) leaves keyboard and screen-reader users in the dark.
- **Typography Consistency:** Monospace font styling must be rigorously enforced for code blocks. Specifying proportional fonts like `Space Grotesk` ahead of monospace is a common layout bug that degrades visual hierarchy.

---

## Appendix: surfaces reviewed

- `crates/core/src/connection.rs` (PrinterConnectionConfig and validate method)
- `crates/adapters/src/lib.rs` (PrinterAdapter trait and structures)
- `crates/adapters/src/factory.rs` (PrinterAdapterFactory build implementation)
- `crates/sdk/src/lib.rs` (Conformance test harness and mocks)
- `crates/cli/src/main.rs` & `crates/cli/src/mcp.rs` (CLI parser and MCP tool definitions)
- `crates/rest/src/main.rs` (Axum REST API routing)
- `index.html` (Landing page layout, CSS styling, and tab switcher JS)
- `user_manual.html` (User manual content and CSS grid layout)
- `architecture.html` (Architecture specification layout)
- `api_reference.html` (API reference layout)

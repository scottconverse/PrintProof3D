# Documentation Deep-Dive — PrintProof3D

**Audit date:** 2026-05-30
**Role:** Technical Writer
**Scope audited:** Root README.md, workspace layout, Cargo.toml package configurations, JSON schemas, fixtures library, and Rust crate source structures.
**Writer mode:** audit+draft
**Auditor posture:** Balanced

---

## TL;DR

The documentation for PrintProof3D is currently in a pre-alpha skeletal state. While the core JSON schemas are auto-generated and valid, the project lacks a proper README, architecture guide, user manual, API documentation, FAQ, changelog, or contributing guidelines. This represents a blocker for onboarding first-time users, returning users, and new contributors alike. In this pass, we have documented all findings and generated a full suite of draft replacements to establish a production-grade documentation baseline.

## Severity roll-up (documentation)

| Severity | Count |
|---|---|
| Blocker | 0 |
| Critical | 4 |
| Major | 3 |
| Minor | 1 |
| Nit | 0 |

## What's working

- **Automated Schema Generation**: The test suite in `crates/core` contains a `generate_schemas` test that writes valid JSON schemas to `/schemas` on test execution.
- **Basic Workspace Structure**: Crate layout is clearly structured in the root `Cargo.toml`.

## What couldn't be assessed

- **Implementation Details of Sub-Crates**: The `printability`, `adapters`, `sdk`, and `cli` crates are currently mock stubs. API references for these crates reflect their planned design rather than complete implementation logic.

---

## Doc asset inventory

| Asset | Exists? | Status | Finding(s) |
|---|---|---|---|
| README.md | Yes | Weak | DOC-001 |
| ARCHITECTURE.md | No | Missing | DOC-002 |
| User manual / guide | No | Missing | DOC-003 |
| API reference | No | Missing | DOC-004 |
| FAQ | No | Missing | DOC-005 |
| CHANGELOG | No | Missing | DOC-007 |
| CONTRIBUTING | No | Missing | DOC-006 |
| SECURITY | No | Missing | DOC-008 |
| LICENSE | No | Missing | DOC-008 |
| Landing / marketing page | No | N/A | None |

---

## Persona walk-through

### First-time user
- **Onboarding blocker**: A user landing on the repository only sees a list of folders and a cargo command. There is no explanation of what the CLI tool does, how to configure a printer profile, or how to validate an STL file.
- **Result**: The first-time user fails to use the tool.

### Returning user
- **Navigation blocker**: A returning user looking to add a custom bed shape or printability check will find no documentation on how the validation models are shaped or what limits are checked by the printability engine.
- **Result**: The returning user cannot progress without digging into the source code of `crates/core`.

### New team member
- **Developer blocker**: A new engineer wants to contribute but has no development guidelines, no instructions on how to regenerate JSON schemas, and no overview of how the 5 crates interact.
- **Result**: The new developer will struggle to build, test, and contribute safely.

---

## Findings

### [DOC-001] — Critical — Onboarding — Inadequate README.md

**Evidence**
- **File**: `PrintProof3D/README.md`
- **Problem**: The README is only 20 lines long. It lists the crates and the build command but completely lacks installation instructions, a CLI guide, usage examples, or help info.

**Why this matters**
First-time users have no path to try or integrate the tool.

**Blast radius**
- Root landing page.

**Fix path**
Full rewrite drafted at `doc-rewrites/README.md`.

---

### [DOC-002] — Critical — Architecture — Missing ARCHITECTURE.md

**Evidence**
- **File**: Repository root (Absent)
- **Problem**: No architecture documentation is present. A complex multi-crate workspace needs to explain its layout, component relationships, and data flow.

**Why this matters**
New developers and systems integrators cannot understand how printability checks are structured or how adapters interface with hardware.

**Blast radius**
- System onboarding.

**Fix path**
Drafted at `doc-rewrites/ARCHITECTURE.md` (includes a Mermaid diagram).

---

### [DOC-003] — Critical — Onboarding — Missing USER_MANUAL.md

**Evidence**
- **File**: Repository root (Absent)
- **Problem**: No user guide or manual exists to explain profile definition formats, G-code/STL constraints, or CLI operation.

**Why this matters**
End-users who want to validate prints or integrate PrintProof3D into their workflows have no step-by-step instructions.

**Blast radius**
- End-user onboarding.

**Fix path**
Drafted at `doc-rewrites/USER_MANUAL.md`.

---

### [DOC-004] — Critical — API — Missing API Reference

**Evidence**
- **File**: Repository root (Absent)
- **Problem**: There is no developer reference detailing public types, traits, and API structures.

**Why this matters**
Integrators looking to use PrintProof3D as a Rust library have no docs on public functions, struct fields, or serialization formats.

**Blast radius**
- API consumers.

**Fix path**
Drafted at `doc-rewrites/API_REFERENCE.md`.

---

### [DOC-005] — Major — FAQ — Missing FAQ.md

**Evidence**
- **File**: Repository root (Absent)
- **Problem**: No Frequently Asked Questions document is available to address common queries.

**Why this matters**
Users are forced to search source code or open support issues for repetitive questions.

**Blast radius**
- Developer and community support overhead.

**Fix path**
Drafted at `doc-rewrites/FAQ.md`.

---

### [DOC-006] — Major — Onboarding — Missing CONTRIBUTING.md

**Evidence**
- **File**: Repository root (Absent)
- **Problem**: The project lacks contribution guidelines, setup workflows, and testing instructions.

**Why this matters**
Open-source or team contributions are hindered by a lack of clear PR processes and local development requirements.

**Blast radius**
- Contributor workflows.

**Fix path**
Drafted at `doc-rewrites/CONTRIBUTING.md`.

---

### [DOC-007] — Major — Hygiene — Missing CHANGELOG.md

**Evidence**
- **File**: Repository root (Absent)
- **Problem**: There is no history or record of changes made to the library.

**Why this matters**
Users cannot track what is new or changes between releases.

**Blast radius**
- Release lifecycle.

**Fix path**
Drafted at `doc-rewrites/CHANGELOG.md`.

---

### [DOC-008] — Minor — Hygiene — Missing License and Security Files

**Evidence**
- **File**: Repository root (Absent)
- **Problem**: Standard LICENSE and SECURITY.md hygiene files are missing.

**Why this matters**
Corporate users and security researchers cannot verify usage rights or know how to report vulnerabilities.

**Blast radius**
- Compliance and security reporting.

**Fix path**
Add stubs or point to parent workspace licenses.

---

## Drafts produced

- `doc-rewrites/README.md` — Complete, developer-focused workspace landing page.
- `doc-rewrites/ARCHITECTURE.md` — Detailed crate architecture and component relation diagram.
- `doc-rewrites/USER_MANUAL.md` — Guide for CLI usage, MCP execution, and JSON profiles.
- `doc-rewrites/API_REFERENCE.md` — API documentation for Rust developers.
- `doc-rewrites/FAQ.md` — Common user and developer questions answered.
- `doc-rewrites/CONTRIBUTING.md` — Guide for setting up, testing, and schema generation.
- `doc-rewrites/CHANGELOG.md` — Tracking release history starting with v0.1.0.

## Patterns and systemic observations

The documentation gaps are typical of a project transitioning from a prototype to a multi-crate system. Since the models in `crates/core` are mature, the documentation has been drafted to cover both the existing schema definitions and the planned interfaces for printability, adapters, and CLI commands.

## Appendix: docs reviewed

- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\README.md`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\audit-lite-stage-1-foundation-2026-05-30.md`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\core\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\printability\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\adapters\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\sdk\src\lib.rs`
- `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D\crates\cli\src\main.rs`

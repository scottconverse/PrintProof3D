# Playwright Interface Wiring Audit Report

**Project:** PrintProof3D  
**Repo:** `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D`  
**Commit audited:** `b2bc9414a546e7273d70bf0eb124ee31562fcace`  
**Audit date:** 2026-06-04  
**Mode:** Walkthrough audit, no product source changes  
**Evidence directory:** `C:\Users\scott\Documents\Codex\2026-06-01\context-handoff-printproof3d-stage-4-release\outputs\walkthrough-printproof3d-2026-06-04`

## Executive Summary

PrintProof3D's browser dashboard is substantially wired to the underlying REST service. The dashboard loads from the Axum server, profile dropdowns populate from live REST endpoints, STL validation works with a valid bearer token, docs/assets routes resolve, exported report controls become available after validation, and the existing multi-browser acceptance suite passes. The remaining interface risks are not blank-page failures; they are state and contract mismatches: the Validate button enables before required profile inputs are selected, auth/setup failures are rendered as critical validation issues, profile-loading failures are console-only, and documented integration contracts remain thinner or less exact than the system behavior.

## Methodology

Reviewed product intent and workflow expectations from `README.md`, `API_REFERENCE.md`, `RELEASE_CHECKLIST.md`, `index.html`, REST route definitions, and the existing browser acceptance harness.

Executed the repository health gate, including formatting, clippy, workspace tests, dependency audit, release build, model/G-code smoke checks, documentation policy/drift checks, and multi-browser Playwright acceptance tests.

Launched the REST dashboard locally with an explicit test token and used Playwright/Chromium to inspect route wiring, profile fetches, responsive layout, file upload state, auth failure state, and successful validation state.

## Project Gestalt

PrintProof3D is a Rust preflight validation engine for 3D printing workflows. Its main user journeys are:

- CLI validation of STL models and G-code files.
- Browser dashboard validation through a local-loopback REST service.
- REST API integration for external tools and agent workflows.
- Profile inspection/validation and compatibility checks.
- Simulator-only adapter conformance; no physical-printer guarantee is claimed.

The browser dashboard is expected to expose a practical validation console: API endpoint/token setup, file upload, printer/material profile selection, validation submission, visualizer feedback, issue list, raw report output, and report export.

## Findings By Severity

### WLK-001 - High - G2/G3 Arc G-code Validation Can False-Pass Swept Out-Of-Bounds Motion

**Location or route:** CLI and REST G-code validation, surfaced through dashboard G-code validation.  
**Element or workflow involved:** G-code upload and validation result.  
**What the user sees:** A validation report can return `pass` for G-code containing an arc move whose endpoints are inside the printer bounds.  
**What actually happens:** Current G-code validation treats `G2` and `G3` like endpoint-only moves. It does not parse `I`, `J`, or `R` arc geometry, so the swept arc path is not checked.  
**What should happen:** The full swept path of arc moves should be checked against the target printer build volume, or arc commands should fail closed until supported.  
**Evidence:** `crates/printability/src/lib.rs:945-1055`; reproduced with `outputs/arc_out_of_bounds_repro.gcode`, which returned `status: pass` with bounding box `0..10` despite the circular sweep crossing negative X.  
**Likely cause:** The movement parser routes `G0`, `G1`, `G2`, and `G3` through the same coordinate endpoint update branch.  
**Suggested fix:** Implement arc geometry parsing and swept-extrema bounds checks for rectangular and cylindrical beds.  
**Suggested test coverage:** Add fixtures for in-bounds endpoints with out-of-bounds arc sweeps, plus normal in-bounds arc moves.

### WLK-002 - Medium - Validate CTA Enables Before Required Printer/Material Inputs Are Present

**Location or route:** Dashboard `/`.  
**Element or workflow involved:** File upload and Validate Print Job button.  
**What the user sees:** After selecting only an STL file, the Validate button becomes enabled even though no printer or material profile is selected.  
**What actually happens:** The UI readiness check only tests whether `selectedFile !== null`.  
**What should happen:** For STL validation, require file, printer, and material before enabling Validate. For G-code validation, require file and printer, with material optional.  
**Evidence:** `index.html:844-848`; Playwright evidence `walkthrough-evidence.json` shows `"after_file_only": {"validate_disabled": false, "printer_value": "", "material_value": ""}`. REST requires `printer` and `material` for `/validate/model` at `crates/rest/src/main.rs:285-292`.  
**Likely cause:** Readiness state was implemented as a file-only check while server validation requirements are stricter.  
**Suggested fix:** Make readiness conditional on file type and selected/uploaded profile state. Add inline unmet-requirement helper text.  
**Suggested test coverage:** Browser tests for button disabled/enabled state across file-only, file+printer, file+printer+material, and G-code optional-material cases.

### WLK-003 - Medium - Auth Failures Are Displayed As Critical Validation Findings

**Location or route:** Dashboard `/`, protected validation endpoints.  
**Element or workflow involved:** Bearer token failure while validating.  
**What the user sees:** The issue list displays `UNAUTHORIZED_ACCESS` with severity `CRITICAL`.  
**What actually happens:** A setup/auth failure is converted into a synthetic validation report issue.  
**What should happen:** Token/auth/setup failures should be shown as system/setup states, not as print validation findings.  
**Evidence:** `index.html:1457-1460`; Playwright evidence `wrong_auth.issues_text` shows `UNAUTHORIZED_ACCESS`, `CRITICAL`, and `API authentication failed...`.  
**Likely cause:** The dashboard uses the same status/issue rendering path for transport/setup failures and true validation reports.  
**Suggested fix:** Separate `setup_error` or `request_error` UI state from `ValidationReport` rendering.  
**Suggested test coverage:** Browser tests asserting 401 shows setup/auth copy and does not populate the anomaly list as a print validation issue.

### WLK-004 - Medium - Profile Fetch Failures Are Console-Only And Lack UI Recovery

**Location or route:** Dashboard `/`.  
**Element or workflow involved:** Predefined printer/material dropdown loading.  
**What the user sees:** If profile endpoints fail, dropdowns remain in placeholder/custom-upload state without visible explanation or retry.  
**What actually happens:** `fetchPredefinedProfiles()` catches errors and logs to the console only.  
**What should happen:** Users should see a loading state, an error state, and retry/custom-upload guidance.  
**Evidence:** `index.html:872-886`; acceptance logs also show a Firefox console error path: `Failed to fetch predefined profiles, routing local default configs`.  
**Likely cause:** Fetch initialization handles happy path only and treats profile loading as a background convenience rather than a first-class setup state.  
**Suggested fix:** Add visible per-dropdown loading/error/retry states and retry on endpoint changes.  
**Suggested test coverage:** Browser tests for profile endpoints returning success, empty, non-OK, and network failure.

### WLK-005 - Medium - REST/API Documentation Is Not Exact Enough For Integration Clients

**Location or route:** `API_REFERENCE.md`, `USER_MANUAL.md`, `docs/AGENT_PRINTER_VALIDATION.md`.  
**Element or workflow involved:** REST integration and machine-readable validation output.  
**What the user sees:** API docs list endpoints and fields, but response shapes and error payloads are thin; one user-manual endpoint marks required `printer` as optional; agent guide examples use title-case enum values.  
**What actually happens:** The runtime has lowercase schema enums, structured errors, required multipart fields, and response headers that are not fully documented in one place.  
**What should happen:** Integration docs should match runtime contracts exactly.  
**Evidence:** `docs/AGENT_PRINTER_VALIDATION.md:143`, `180`; `schemas/validation_report.schema.json:121-125`, `343-345`; `USER_MANUAL.md:313`; `crates/rest/src/main.rs:816-819`; `API_REFERENCE.md:449-489`.  
**Likely cause:** Docs are synced structurally but not contract-tested for examples and field requiredness.  
**Suggested fix:** Update docs and generated HTML with exact enum casing, required/optional fields, response shapes, status codes, headers, and multipart examples.  
**Suggested test coverage:** Add docs-example contract checks against schemas and REST test expectations.

### WLK-006 - Low - Reduced-Motion Preferences Are Not Honored

**Location or route:** Dashboard `/`.  
**Element or workflow involved:** CSS transitions, spinner animation, continuous visualizer render loop.  
**What the user sees:** Motion remains active even when the browser/user preference requests reduced motion.  
**What actually happens:** No `prefers-reduced-motion` CSS or visualizer render-mode adjustment was found.  
**What should happen:** Nonessential animations should be disabled or reduced for users requesting reduced motion.  
**Evidence:** `index.html` animation/transition definitions and no matching reduced-motion media query.  
**Likely cause:** Accessibility polish not yet added for motion preferences.  
**Suggested fix:** Add a `prefers-reduced-motion` block and avoid continuous rendering when idle if feasible.  
**Suggested test coverage:** CSS/source assertion or Playwright media-emulation check.

## Missing Or Partial Features

- Full swept-path validation for `G2`/`G3` arc motion is missing even though docs describe stateful `G0`-`G3` path auditing.
- Dashboard setup-state handling is partial: it handles happy path and auth failure, but setup errors are mixed into validation findings.
- Profile loading lacks visible failure/retry states.
- REST integration docs are present but not yet contract-complete for response/error examples.

## Backend Or System Capabilities Not Surfaced

- REST response headers such as `X-Validation-Status` are implemented but not clearly surfaced in API docs.
- Structured REST error payloads are implemented and tested but not documented with examples.
- The UI surfaces validation and export well, but does not expose profile inspection/validation endpoints as first-class dashboard actions.

## Confusing Or Misleading UI

- File-only readiness implies validation can begin before required profiles are selected.
- Auth failure appears as a critical validation anomaly, which can mislead users into treating a setup problem as a print risk.
- Dropdowns can fail to populate without visible recovery guidance.

## Broken Or Suspicious Wiring Map

| UI element or workflow | Expected system connection | Actual connection | Status | Evidence |
| --- | --- | --- | --- | --- |
| Root dashboard `/` | Serve interactive validation UI | 200 HTML, profiles fetch, assets load | Working | `walkthrough-evidence.json` routes |
| Docs links | Serve user manual, API reference, architecture | All return 200 | Working | `walkthrough-evidence.json` routes |
| Profile dropdowns | Fetch `/profiles/printers` and `/profiles/materials` | Populate options on happy path | Working, failure state incomplete | `printer_options: 8`, `material_options: 3`; `index.html:872-886` |
| File upload -> Validate readiness | Enable only once required inputs are present | Enables after file only | Partially wired | `after_file_only.validate_disabled: false` |
| Wrong bearer token | Show setup/auth failure | Shows alert and critical validation issue | Misleading wiring | `wrong_auth.issues_text` |
| Valid STL validation | POST `/validate/model` and render/export report | Works with correct token/profiles | Working | `success-validation.png`; export button visible |
| Missing route | Return 404 | Returns 404 | Working | `walkthrough-evidence.json` routes |

## Test Assessment

The existing test posture is strong for a local developer engine. The health gate passed:

- Git cleanliness gate.
- Documentation policy scan.
- Documentation drift check.
- Formatting and clippy with denied warnings.
- Workspace unit/integration tests.
- Dependency audit with one allowed transitive warning.
- Release CLI build.
- Model and G-code smoke checks.
- Multi-browser browser acceptance tests.

Existing Playwright tests prove profile loading, auth rejection, successful validation, WebGL/fallback behavior, export, spinner/reset state, token autocomplete, rectangular/cylindrical client-side volume precheck, and responsive layout across Chromium, Firefox, and WebKit.

High-value missing tests:

- G2/G3 swept arc false-negative regression tests.
- Plugin-mutated report invariant regression test.
- UI readiness tests for required profile selections.
- UI setup/auth error tests that assert errors are not rendered as validation anomalies.
- Profile endpoint failure and retry-state tests.
- Docs contract tests for schema enum examples and required multipart fields.

## Recommended Repair Plan

### Immediate Blockers

None found in the walkthrough. The interface launches, routes resolve, validation works, and health is green.

### Core Wiring Fixes

1. Fix G2/G3 arc path validation.
2. Revalidate plugin-mutated report invariants.
3. Fix Validate CTA readiness to match REST requirements.
4. Separate setup/auth failures from validation report findings.
5. Add visible profile-fetch error/retry states.

### Feature Completion

1. Expand REST API docs to exact contract-level examples.
2. Consider exposing profile inspect/validate actions in the dashboard if the dashboard is intended as the full REST console, not only a validation surface.

### UI/UX Cleanup

1. Add reduced-motion support.
2. Harmonize dashboard/docs navigation.

### Test Coverage

Add regression coverage for the five core wiring fixes above, prioritizing arc validation and plugin invariant enforcement.

## Confidence And Gaps

High confidence that the primary dashboard, REST, docs routes, and acceptance-tested workflows are wired and usable. High confidence that the arc validation false-negative and UI readiness/auth-state findings are real, because they were reproduced or directly observed against current source/runtime.

Not assessed: physical printer hardware behavior, long-running load, concurrent uploads, native mobile browsers, real external integrations, and full MCP stdin/stdout interaction beyond existing tests.

## Appendix

Key evidence artifacts:

- `C:\Users\scott\Documents\Codex\2026-06-01\context-handoff-printproof3d-stage-4-release\outputs\walkthrough-printproof3d-2026-06-04\walkthrough-evidence.json`
- `C:\Users\scott\Documents\Codex\2026-06-01\context-handoff-printproof3d-stage-4-release\outputs\walkthrough-printproof3d-2026-06-04\dashboard-1440.png`
- `C:\Users\scott\Documents\Codex\2026-06-01\context-handoff-printproof3d-stage-4-release\outputs\walkthrough-printproof3d-2026-06-04\wrong-auth-state.png`
- `C:\Users\scott\Documents\Codex\2026-06-01\context-handoff-printproof3d-stage-4-release\outputs\walkthrough-printproof3d-2026-06-04\success-validation.png`

Notable runtime evidence:

- `/`, `/docs/user_manual`, `/docs/api_reference`, `/docs/architecture`, `/profiles/printers`, `/profiles/materials`, and `/assets/three.min.js` returned 200.
- `/missing-route` returned 404.
- Dashboard had no horizontal overflow at 320x900, 768x900, 1280x720, or 1440x900.
- Wrong-token validation produced a visible auth alert but also inserted a `CRITICAL` validation anomaly.
- Valid STL validation with correct token enabled report export.

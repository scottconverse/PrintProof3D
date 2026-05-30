# UI/UX & DX Deep-Dive — PrintProof3D

**Audit date:** 2026-05-30
**Role:** Senior UI/UX Designer
**Scope audited:** PrintProof3D core data models, JSON schemas, CLI implementation, documentation examples, and SDK adapters.
**Auditor posture:** Balanced

---

## TL;DR

PrintProof3D has successfully resolved the most critical Developer Experience (DX) and onboarding blockers identified in the initial Stage 1 audit. Notably, CLI argument parsing using `clap` is operational, the CLI subcommands `validate-model` and `validate-gcode` now match the documentation, the library usage and configuration examples in the manual and README are fully corrected and compile/parse properly, and the CLI now exits with a non-zero code on validation warnings or failures (ensuring pipeline integrations function correctly). 

However, several Major and Minor DX issues remain active. These include custom bed shapes being restricted to freeform text strings (preventing visual 3D rendering), the assumed slicer settings being modeled as a freeform JSON blob (blocking structured UI displays), and a nomenclature mismatch across protocols and adapters. Additionally, a new Minor inconsistency has been identified in the user manual's description of the planned MCP server subcommand.

---

## Severity roll-up (UX/DX)

| Severity | Count | Status Change (vs Previous) |
|---|---|---|
| Blocker | 0 | Unchanged |
| Critical | 0 | -3 (All resolved) |
| Major | 2 | -2 (UX-008, UX-009 resolved) |
| Minor | 3 | +1 (UX-014 added, UX-007 and UX-010 active) |
| Nit | 0 | Unchanged |
| **Total Outstanding** | **5** | **-5 total active findings** |

---

## What's working

- **Corrected Onboarding Commands:** The `README.md` and `USER_MANUAL.md` documentation has been updated to use the correct `validate-model` and `validate-gcode` subcommands, resolving immediate quickstart failures.
- **Valid Library Usage Examples:** The code snippets in the README properly instantiate the `BuildVolume` enum, allowing developers to copy-paste Rust code that compiles successfully.
- **Valid Presets and Schemas:** Material preset files and examples now use `"low"` risk levels instead of the outdated and syntax-invalid `"easy"` option, preventing JSON parsing crashes.
- **CLI Exit Code Gating:** The CLI now correctly returns a non-zero exit code (`1`) when a validation report contains warnings or failures, enabling integration into Git hook pipelines.
- **Build Volume & Bed Shape Invariant Validation:** `PrinterProfile::validate()` now checks for geometric incompatibilities (such as circular bed shapes paired with rectangular volumes) and rejects invalid profiles.
- **Clap CLI Parser Deployed:** Running the CLI now provides structured argument parsing, subcommands, and basic error reporting, replacing the non-functional stub.
- **Cylindrical Build Volumes:** Delta-style cylindrical printers can now be modeled natively via `BuildVolume::Cylindrical`.
- **Rich Spatial Validation Locations:** Frontends can now draw red bounding boxes or highlight specific mesh facets using the new `LocationGeometry` tagged enum.
- **Improved Material Copy:** Material warp risks are now classified under a clear `RiskLevel` (`low`, `medium`, `high`) rather than the confusing `Difficulty` enum.

---

## What couldn't be assessed

- **Actual Execution Times of Printability Checks:** The printability check is still a mock returning a passing report with empty issues. Heavy files (e.g., 500k polygon STLs) will require background loading states, which cannot be tested yet.
- **Tauri / Web Frontend States:** Visual styling and interactive layout of validation reports could not be tested directly due to the lack of a graphical client.

---

## First impressions

- **CLI Interface:** Running `printproof3d --help` now provides a standard, descriptive help menu. Executing the quickstart commands now runs successfully.
- **Onboarding and Configuration:** The user manual and README are highly detailed, and the copy-paste JSON configurations and code examples are now fully aligned with the Rust struct changes, preventing immediate syntax and compiler crashes for developers trying to adopt the SDK.

---

## Journey walkthroughs

### Journey 1: Slicer Developer integrates PrintProof3D validation into a 3D Web UI
1. The developer downloads the exported schemas. They see that `IssueLocation` has `geometry: Option<LocationGeometry>`, which lets them successfully render points, bounding boxes, or highlight specific triangles in Three.js!
2. However, they check `sliced_settings_assumed` and find it is still a freeform, undocumented JSON object. They must guess the keys or hardcode values to show an assumed slicing parameters card.
3. The developer copies the example material profile from the `USER_MANUAL.md` to bootstrap their default preset. This time, the preset parses successfully because the manual correctly uses `"warp_risk": "low"`.

### Journey 2: Printer Operator configures a Delta printer profile
1. The operator configures a Cylindrical build volume, which works perfectly.
2. However, they select "Custom" for their non-standard triangular bed shape. The schema forces them to input a raw string `Custom("triangular")` which provides no vertex data to draw in their 3D scene.

### Journey 3: DevOps Engineer integrates PrintProof3D into a Git pre-commit hook
1. The engineer runs `printproof3d validate-model --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json` as shown in the README, and the command executes successfully.
2. If they run it on an invalid profile, the validation fails, and the CLI exits with code `1`, causing the pre-commit hook script to register the failure and successfully block the push.

---

## Findings

> **Finding ID prefix:** `UX-`
> **Categories:** Visual hierarchy / Copy / State / Accessibility / Responsive / Journey / Pattern / Motion / IA

### [UX-003] — Major — Information Architecture / Pattern — Custom bed shapes are defined by a raw `String` preventing visual 3D rendering

*Status:* **Active**

**Evidence**
- File: `crates/core/src/lib.rs:L12-16`
```rust
pub enum BedShape {
    Rectangular,
    Circular,
    Custom(String),
}
```

**Why this matters**
A modern 3D printer UI renders the print bed visually to orient models. A raw `String` (e.g. `Custom("triangular")`) does not provide enough geometric metadata for a 3D viewer. This leaves the user with a default rectangular grid, creating a layout disconnect.

**Blast radius**
- `BedShape` enum in `crates/core/src/lib.rs` and its schema.
- User-facing: 3D frontend viewer integrations (Tauri or Web UI) will lack geometry representation.

**Fix path**
Replace `Custom(String)` with a struct defining 2D boundary points or a 3D mesh model link:
```rust
pub enum BedShape {
    Rectangular,
    Circular,
    Custom {
        boundary_points: Vec<(f32, f32)>,
        mesh_file_path: Option<String>,
    },
}
```

---

### [UX-004] — Major — State / Pattern — `sliced_settings_assumed` is a freeform JSON value, blocking structured UI displays

*Status:* **Active**

**Evidence**
- File: `crates/core/src/lib.rs:L361`
```rust
pub sliced_settings_assumed: Option<serde_json::Value>,
```

**Why this matters**
Slicer settings determine print success. By using a freeform JSON blob, there are no schema guarantees, meaning frontend developers cannot build structured, interactive comparison tables. Users are left with a raw JSON dump in the UI.

**Blast radius**
- `ValidationReport` struct and its schema.
- User-facing: Any UI rendering comparison cards or assumed parameters.

**Fix path**
Define a structured `SlicedSettings` struct in `crates/core` with common slicing parameters (e.g. `layer_height`, `infill_density`, `print_speed`) and use it instead of `serde_json::Value`.

---

### [UX-007] — Minor — Copy / Consistency — Nomenclature mismatch between protocol names, firmware flavors, and adapter identifiers

*Status:* **Active**

**Evidence**
- `crates/core/src/lib.rs:L21-24`:
  ```rust
  pub enum ProtocolFamily {
      Klipper,
      MarlinSerial,
      // ...
  }
  ```
- `crates/adapters/src/lib.rs:L7`:
  ```rust
  pub fn list_adapters() -> Vec<&'static str> {
      vec!["moonraker", "octoprint", "marlin"]
  }
  ```

**Why this matters**
A user selecting `Klipper` as their protocol will not see a direct mapping to the connection adapter named `"moonraker"`, and selecting `MarlinSerial` maps to `"marlin"`. This causes confusion when writing integrations.

**Blast radius**
- `ProtocolFamily` enum and adapter listings.

**Fix path**
Standardize naming. Either rename `ProtocolFamily::Klipper` to `ProtocolFamily::Moonraker`, or rename the adapter from `"moonraker"` to `"klipper"`. Similarly, align `MarlinSerial` and `"marlin"`.

---

### [UX-010] — Minor — CLI DX — Arbitrary and non-intuitive short flag `-a` for `--material`

*Status:* **Active**

**Evidence**
- `crates/cli/src/main.rs:L30` and `L48`:
  ```arg
  #[arg(long, short = 'a')]
  material: PathBuf,
  ```

**Why this matters**
The short argument `-a` for `--material` has poor discoverability. Developers expecting standard options like `-f` (filament) or `-t` (maTerial) will find `-a` unintuitive.

**Blast radius**
- `crates/cli/src/main.rs` argument parsing.

**Fix path**
Either remove the short flag `-a` (relying solely on `--material`) or rename it to `-f` (for filament) or `-t` (for material).

---

### [UX-014] — Minor — Copy / DX — Misleading user manual mention of inactive `mcp` subcommand

*Status:* **Active (New)**

**Evidence**
- `USER_MANUAL.md:L117-121`:
  ```markdown
  > [!NOTE]
  > MCP server integration is a planned Stage 2 feature. In the current Stage 1 release, the server protocol logic and its subcommands (such as `printproof3d mcp`) are mock interfaces and not active.
  ```
- In `crates/cli/src/main.rs`, the `Commands` enum does not contain an `mcp` subcommand.

**Why this matters**
The manual claims the `printproof3d mcp` subcommand is an inactive/mock interface. A user attempting to execute `printproof3d mcp` will receive a CLI parsing error from `clap` (`error: unrecognized subcommand 'mcp'`) instead of an informative notice, causing confusion.

**Blast radius**
- `USER_MANUAL.md` and `audit-team-stage-1/doc-rewrites/USER_MANUAL.md`.

**Fix path**
Either add a mock `mcp` subcommand in `crates/cli/src/main.rs` that returns a friendly notice ("MCP server integration is planned for Stage 2. This subcommand is not yet active."), or update the user manual to clarify that the subcommand is completely omitted from the CLI binary in this stage.

---

## Resolved Historical Findings

### [UX-001] — RESOLVED — CLI binary is a non-functional stub
*Status:* **Resolved** in `crates/cli/src/main.rs` by implementing a clap-based argument parser with subcommands `validate-model` and `validate-gcode`. It now prints usage help and validates paths.

### [UX-002] — RESOLVED — Circular Build Volume is represented as a Rectangular Box in `BuildVolume`
*Status:* **Resolved** in `crates/core/src/lib.rs` by refactoring `BuildVolume` into a tagged enum containing `Rectangular` and `Cylindrical` variants.

### [UX-005] — RESOLVED — `IssueLocation` uses a single 3D point (`x`, `y`, `z`) which is insufficient
*Status:* **Resolved** in `crates/core/src/lib.rs` by adding `geometry: Option<LocationGeometry>`, supporting `Point`, `BoundingBox`, and `Triangles` variants for precise highlights.

### [UX-006] — RESOLVED — MaterialProfile "warp_risk" uses confusing Difficulty enum
*Status:* **Resolved** in `crates/core/src/lib.rs` by refactoring the field to use the `RiskLevel` enum (with `Low`, `Medium`, `High` values), eliminating the confusing `"easy"` warp risk classification.

### [UX-008] — RESOLVED — No compatibility validation between `bed_shape` and `build_volume` shape
*Status:* **Resolved** in `crates/core/src/lib.rs` by enforcing compatibility rules in `PrinterProfile::validate()` (ensuring Circular beds require Cylindrical build volumes, and Rectangular beds require Rectangular build volumes).

### [UX-009] — RESOLVED — CLI exits with success code (0) even when validation fails or warnings are found
*Status:* **Resolved** in `crates/cli/src/main.rs` by returning exit code `1` when warnings or failures are present in the report.

### [UX-011] — RESOLVED — CLI subcommand name mismatch (`validate` vs `validate-model` / `validate-gcode`)
*Status:* **Resolved** in `README.md` and `USER_MANUAL.md` (and doc-rewrites) by correcting the documented commands to match the actual subcommands.

### [UX-012] — RESOLVED — Outdated library usage example for `BuildVolume` in README.md
*Status:* **Resolved** in `README.md` (and doc-rewrites) by updating the Rust snippet to use the new `BuildVolume::Rectangular` enum variant structure.

### [UX-013] — RESOLVED — Outdated example configurations in USER_MANUAL.md
*Status:* **Resolved** in `USER_MANUAL.md` (and doc-rewrites) by changing the material configuration parameters to match the new `RiskLevel` enum (e.g. using `"low"` instead of `"easy"`).

---

## States audit matrix

| Component / page | Default | Loading | Empty | Error | Partial | Notes |
|---|---|---|---|---|---|---|
| CLI `validate-model` | ✓ | ✗ | — | ✓ | — | No status/loading feedback for large meshes |
| CLI `validate-gcode` | ✓ | ✗ | — | ✓ | — | No status/loading feedback for large G-code |
| Printer Profile Schema | ✓ | — | ✗ | ✓ | — | Empty profile has no default fallback |
| Material Profile Schema | ✓ | — | ✗ | ✓ | — | Empty profile has no default fallback |

*Key:* ✓ = Handled / Present; ✗ = Unhandled / Missing; — = Not Applicable.

---

## Accessibility snapshot

- **Keyboard navigation:** Not Applicable (CLI terminal utility).
- **Focus visibility:** Not Applicable.
- **Color contrast:** Plain text JSON stdout/stderr is returned. Colors are not hardcoded, preventing low contrast conflicts with dark/light shell themes.
- **Screen reader labeling:** CLI standard output stream is accessible, but reading large, flat JSON reports in terminal output can be challenging for screen readers compared to formatted text outputs.
- **Reduced motion:** Not Applicable.
- **Touch target size:** Not Applicable.

---

## Patterns and systemic observations

1. **Nomenclature Inconsistency:** Protocol names, firmware flavors, and adapter identifiers span multiple crates (`core` and `adapters`) and use varying identifiers (e.g. `Klipper` vs `"moonraker"`, `MarlinSerial` vs `"marlin"`). Establishing a unified nomenclature dictionary is key before scaling Stage 2 integrations.
2. **Schema Rigidity:** Using `serde_json::Value` (for assumed settings) or `String` (for custom bed shapes) weakens the schema contracts. Strongly typing these parameters will allow third-party client integrations to build robust visual interfaces.

---

## Appendix: surfaces reviewed

- **Crate source files:**
  - `crates/core/src/lib.rs` (Data models, schemas, and validators)
  - `crates/cli/src/main.rs` (Clap subcommands, exit code logic, and file loader)
  - `crates/printability/src/lib.rs` (Validators traits)
  - `crates/adapters/src/lib.rs` (Adapter listings)
- **Documentation:**
  - `README.md`
  - `USER_MANUAL.md`
  - `audit-team-stage-1/doc-rewrites/README.md`
  - `audit-team-stage-1/doc-rewrites/USER_MANUAL.md`
- **Schemas:**
  - `schemas/printer_profile.schema.json`
  - `schemas/material_profile.schema.json`
  - `schemas/validation_report.schema.json`

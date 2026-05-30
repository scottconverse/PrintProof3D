# UI/UX Deep-Dive — PrintProof3D

**Audit date:** 2026-05-30
**Role:** Senior UI/UX Designer
**Scope audited:** The PrintProof3D data models, JSON schemas, CLI stub, and SDK adapters.
**Auditor posture:** Balanced

---

## TL;DR

The PrintProof3D Stage 1 codebase is a well-structured Rust workspace that compiles cleanly, generates JSON schemas automatically, and defines core profiles. However, from a UX/DX perspective, the interface surfaces—consisting of the Command Line Interface (CLI) and the JSON schemas that power frontend profile editors and 3D previewers—suffer from several critical and major gaps. 
First, the CLI is currently a non-functional stub that lacks argument parsing, help descriptions, and output formatting, representing a **Critical** CLI UX blocker. 
Second, the data models exhibit **Major** information architecture and usability issues: circular print beds are forced into rectangular coordinate volumes, custom bed shapes are restricted to raw text strings (preventing visual 3D bed rendering), and spatial validation errors are located via a single point `(x, y, z)` rather than bounding boxes or toolpath segments, which renders 3D visualization of issues (like overhangs) virtually impossible. 
Finally, the copy is marred by semantic confusion, notably using a three-level "Difficulty" enum (`Easy`, `Medium`, `Hard`) to describe `warp_risk`, which results in the nonsensical classification of "Easy warp risk." 
Addressing these schema-level and CLI-level gaps now, before the engine is integrated, is vital to ensure that downstream clients can render accessible, intuitive, and modern 3D printing interfaces.

---

## Severity roll-up (UX)

| Severity | Count |
|---|---|
| Blocker | 0 |
| Critical | 1 |
| Major | 5 |
| Minor | 3 |
| Nit | 1 |

---

## What's working

- **Robust serialization and schema generation:** The core serialization definitions in `crates/core` are cleanly defined, derive serialization/deserialization correctly, and dynamically export JSON schemas during tests. This ensures that frontend clients have access to up-to-date schema definitions for validation.
- **Strong model verification foundations:** The mock serialization tests in `crates/core/src/lib.rs` are detailed, verifying that all fields round-trip correctly, preventing data loss or parsing errors at the API boundary.
- **Logical profile structures:** The separating of printer-specific capabilities (nozzle sizes, temperature boundaries, enclosure presence, connectivity protocols) and material-specific attributes (extrusion temperatures, cooling, warp risk) is a clean and standard domain model separation.
- **Clean protocol enum lists:** The `ProtocolFamily` and `FirmwareFlavor` enums provide a broad list of common 3D printing standards (Klipper, OctoPrint, Prusa, Marlin, RepRapFirmware, Bambu, etc.), which facilitates building direct connection lists in UI dropdowns.

---

## What couldn't be assessed

- **Interactive visual rendering of validation reports:** Since there is currently no graphical client or frontend attached to this engine, the visual look-and-feel of the reports, their layout, colors, and interactive states could only be analyzed from the perspective of schema capabilities, rather than an active Web/Tauri runtime.
- **CLI progress indicators and terminal interactivity:** Since the CLI has no interactive behavior implemented, terminal scroll, spinner animations during mesh calculation, and progress bars could not be evaluated.
- **Actual execution times of printability checks:** The printability check is currently a mock returning `"ok"`. Long-running checks for complex STL meshes (e.g. checking a 500,000 polygon STL for manifold errors) will require background progress/loading states in the CLI and API, which cannot yet be verified.

---

## First impressions

- **CLI Interface:** Running `printproof3d` displays a single static line: `PrintProof3D CLI version 0.1.0`. There are no instructions, no interactive command help, and no response to flags like `-h` or `--help`. For a developer trying to evaluate the tool, the entry point is a dead end.
- **Schemas and Data Models:** Reading the schemas reveals a clean initial draft, but immediate UX roadblocks appear for a designer planning a web frontend. How do we draw a Delta printer's round build volume? How do we show the user where an overhang is if we only have a single coordinate point? Why is ABS's warp risk labeled "Easy"? The developer experience (DX) and subsequent user interface (UI) will suffer due to these schema-level constraints.

---

## Journey walkthroughs

### Journey 1: Slicer Developer integrates PrintProof3D validation into a 3D Web UI
1. The developer downloads the exported schemas from `/schemas` to auto-generate TypeScript types for their React 3D viewer.
2. **Gap:** The developer inspects `IssueLocation` and discovers it only provides `x`, `y`, `z` as optional floats and a `region` string. They realize they cannot draw a highlighting box or highlight specific mesh faces in Three.js/Babylon.js because the exact bounds of the issue are not defined in the schema. (See **UX-005**).
3. **Gap:** The developer tries to parse `sliced_settings_assumed` to display a "Parameters Used for Analysis" table. They find it is typed as a freeform, undocumented JSON object, meaning they must guess the keys or hardcode undocumented values. (See **UX-004**).

### Journey 2: Printer Operator configures a Delta (circular bed) printer profile
1. The operator opens the printer profile setup screen in the UI.
2. They select "Circular" as the bed shape.
3. The UI requests the build volume. Because `BuildVolume` strictly defines `x`, `y`, `z`, the user has to input the bounding box width/depth for `x` and `y` (e.g. `220` and `220` for a 220mm diameter bed).
4. **Gap:** The user is confused by the request for `x` and `y` dimensions for a circular printer, as they think in terms of `radius` or `diameter`.
5. **Gap:** When the 3D model is loaded, the UI renders a rectangular bounding box prism on screen. The user tries to place a print near the corners of this box, only to receive validation warnings that it is out-of-bounds, as the circular bed cut-off is not visually represented in the workspace. (See **UX-002**).

---

## Findings

### [UX-001] — Critical — Pattern / Journey / CLI UX — CLI binary is a non-functional stub lacking argument parsing, help commands, and formatted outputs

**Evidence**
`crates/cli/src/main.rs:1-6`:
```rust
fn main() {
    println!("PrintProof3D CLI version 0.1.0");
}
```
The dependency `clap` is declared in `crates/cli/Cargo.toml` but completely unused in `main.rs`.

**Why this matters**
As a CLI tool, the terminal user interface *is* the primary product interface at this stage. Currently, developers and operators have no way to run validation commands, pass profile paths, or get feedback. There is no help interface (`--help` or `-h`) or version information query, which violates basic CLI usability standards. This is a dead end for any user attempting to evaluate the tool.

**Blast radius**
- The `crates/cli` crate.
- User-facing: Anyone attempting to run the CLI gets a static string with no way to progress.

**Fix path**
Implement CLI commands using `clap`. At a minimum, support:
- `printproof3d validate --file <file> --printer <profile> --material <profile>`
- `--help` and `-h` flags with clear command descriptions.
- `--format <json|text>` to allow machine-readable script integration and human-readable terminal output.
- Return exit codes: `0` (Success/Pass), `1` (Warnings/Fail issues found), `2` (CLI syntax or runtime errors).

---

### [UX-002] — Major — Information Architecture / Journey — Circular Build Volume is represented as a Rectangular Box in `BuildVolume`

**Evidence**
`crates/core/src/lib.rs:49-53`:
```rust
pub struct BuildVolume {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
```
This rectangular structure is utilized in `PrinterProfile::build_volume` and `ModelMetadata::bounding_box` regardless of the `BedShape` (rectangular or circular).

**Why this matters**
Delta printers and polar coordinates systems use cylindrical build volumes defined by diameter/radius and height. Requiring users to define circular volumes as `x`, `y`, `z` Cartesian coordinates forces them to calculate equivalent bounding boxes (introducing potential error). Furthermore, in a 3D visualization, rendering a cylindrical volume as a rectangular box leads to visual confusion, as the user might believe they can print in the corners when in reality the print head cannot reach them, leading to collision or print failure.

**Blast radius**
- `BuildVolume`, `PrinterProfile`, and `ModelMetadata` structs, and the corresponding JSON schemas.

**Fix path**
Refactor `BuildVolume` to support both rectangular and cylindrical volumes using a tagged enum:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuildVolume {
    Rectangular { x: f32, y: f32, z: f32 },
    Cylindrical { radius: f32, height: f32 },
}
```

---

### [UX-003] — Major — Information Architecture / Pattern — Custom bed shapes are defined by a raw `String` preventing visual 3D rendering

**Evidence**
`crates/core/src/lib.rs:11-15`:
```rust
pub enum BedShape {
    Rectangular,
    Circular,
    Custom(String),
}
```

**Why this matters**
A modern 3D printer UI renders the print bed visually to orient models. A raw `String` (e.g. `Custom("triangular")` or `Custom("belt")`) does not provide enough geometric metadata for a 3D viewer. This leaves the user with a blank or default rectangular grid, creating a layout disconnect.

**Blast radius**
- `BedShape` enum in `core` and its schema.

**Fix path**
Replace `Custom(String)` with a struct that defines the boundary as a list of 2D coordinates (vertices) or links to a 3D model file (STL/OBJ) for the print bed:
```rust
Custom {
    boundary_points: Vec<(f32, f32)>,
    mesh_file_path: Option<String>,
}
```

---

### [UX-004] — Major — State / Pattern — `sliced_settings_assumed` is a freeform JSON value, blocking structured UI displays

**Evidence**
`crates/core/src/lib.rs:163`:
```rust
pub sliced_settings_assumed: Option<serde_json::Value>,
```
And in `validation_report.schema.json:26`, it is defined as `true`.

**Why this matters**
Slicer settings (such as speeds, retraction, temperatures, and layer height) are critical parameters that determine whether a print will succeed. By typing this as a freeform JSON blob, the engine provides no schema guarantees. Frontend developers cannot build structured, interactive "Assumed Slicing Settings" cards or comparison tables. Users are left with a raw JSON dump, reducing visibility into what settings the engine assumed for its printability checks.

**Blast radius**
- `ValidationReport` struct and its JSON schema.

**Fix path**
Define a structured `SlicedSettings` struct in `core` containing common slicing parameters (e.g., `layer_height`, `nozzle_temp`, `bed_temp`, `print_speed`, `retraction_length`, `supports_enabled`) and use it in place of `serde_json::Value`.

---

### [UX-005] — Major — Accessibility / Visual hierarchy — `IssueLocation` uses a single 3D point (`x`, `y`, `z`) which is insufficient for highlighting spatial printability issues in a 3D viewer

**Evidence**
`crates/core/src/lib.rs:139-144`:
```rust
pub struct IssueLocation {
    pub region: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
}
```

**Why this matters**
Validation issues like overhangs, bed adhesion failures, or bridging spans are 3D features, not infinitesimal points. A single coordinate point `(x, y, z)` does not specify the dimensions or scale of the issue. When a frontend 3D viewer attempts to highlight the issue (e.g. wrapping it in a red bounding box or highlighting the affected mesh triangles), a single point is insufficient. Users will see a point in space but won't know the boundaries of the problem area.

**Blast radius**
- `ValidationIssue`, `IssueLocation` structs and schemas.

**Fix path**
Extend `IssueLocation` to include an optional bounding box (`bounding_box: Option<BuildVolume>`) or toolpath segment range, or mesh face indices. E.g.
```rust
pub struct IssueLocation {
    pub region: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub bounding_box: Option<BuildVolume>,
    pub affected_facets: Option<Vec<u32>>,
}
```

---

### [UX-006] — Major — Copy / Semantics — MaterialProfile "warp_risk" uses the confusing `Difficulty` enum instead of a risk-based enum

**Evidence**
`crates/core/src/lib.rs:104`:
```rust
pub warp_risk: Difficulty,
```
where `Difficulty` is `Easy`, `Medium`, `Hard`.

**Why this matters**
"Easy warp risk" is semantically contradictory and confusing. Does "Easy" mean the material is easy to print without warping (low risk), or does it mean warping happens easily (high risk)? For a user configuring or viewing material profiles, this ambiguity can lead to print failures (e.g. printing ABS without an enclosure because they thought "Easy" warp risk meant low risk).

**Blast radius**
- `MaterialProfile` struct and its JSON schema `material_profile.schema.json`.

**Fix path**
Create a dedicated `RiskLevel` enum (e.g., `Low`, `Medium`, `High`) or rename the field to `warp_resistance` or `warp_risk` with a proper enum like `Low`, `Medium`, `High`. For example, `pub warp_risk: RiskLevel,` where `RiskLevel` is `Low`, `Medium`, `High`.

---

### [UX-007] — Minor — Copy / Consistency — Nomenclature mismatch between `ProtocolFamily::Klipper` in printer profiles and `"moonraker"` in connection adapters

**Evidence**
- `crates/core/src/lib.rs` line 20: `Klipper` is defined as a `ProtocolFamily`.
- `crates/adapters/src/lib.rs` line 4: `"moonraker"` is returned as an available adapter name.

**Why this matters**
Klipper is the firmware, and Moonraker is the HTTP API wrapper. A user setting up their printer might select `Klipper` as the connection type, but the SDK expects `moonraker` as the adapter name. This inconsistency makes it harder for a user or developer to map printer profile configurations to connection adapters.

**Blast radius**
- `ProtocolFamily` enum and adapter listings.

**Fix path**
Standardize the naming. Either include `moonraker` in the `ProtocolFamily` enum (since Klipper connection is technically via Moonraker API), or map the Klipper variant to the Moonraker adapter name under the hood, or rename the adapter to `klipper` for parity.

---

## Conclusion
All UI/UX findings have been documented, and fixing these data schemas and CLI parameters is the first critical step before building out engine features or graphical UI dashboards.

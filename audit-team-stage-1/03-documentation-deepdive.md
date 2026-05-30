# Documentation Deep-Dive — PrintProof3D

**Audit date:** 2026-05-30
**Role:** Technical Writer
**Scope audited:** Root README.md, ARCHITECTURE.md, USER_MANUAL.md, API_REFERENCE.md, FAQ.md, CONTRIBUTING.md, CHANGELOG.md, and workspace Rust structures.
**Writer mode:** audit+draft
**Auditor posture:** Balanced

---

## TL;DR

Following up on the previous documentation audit, we performed a thorough verification of the root documentation assets. Out of the 8 issues identified previously, **6 have been fully resolved** (including CLI command sync, enum code compilation, JSON profile deserialization tags, difficulty level mappings, and git hook instructions), **1 has been partially resolved** (traits are documented in the API reference, but helper types are omitted), and **1 remains unresolved** (missing root LICENSE and SECURITY.md files).

However, our deep-dive verification has identified **one new blocker-level onboarding issue** and **two new major-level API reference issues**:
1. The documented CLI validation examples reference a `profiles/` directory and profiles (`profiles/prusa_mk4.json`, `profiles/pla.json`) that do not exist in the repository.
2. The `PrinterProfile` field list in the API reference omits 9 of the 24 fields present in the source code (though the code example correctly instantiates them).
3. The API reference documents the `PrinterAdapter` trait but omits definitions for its returned types `PrinterTelemetry` and `AdapterError`.

This updated report outlines the status of the previous issues and details the new findings. To ensure onboarding works out-of-the-box and the API reference is accurate, we have provided draft updates in the report.

---

## Severity roll-up (documentation)

| Severity | Count | What it means |
|---|---|---|
| Blocker | 1 | Code examples or commands cannot be run as written. |
| Critical | 0 | (Pre-existing skeleton gaps have been filled). |
| Major | 2 | API references omit core fields or helper types. |
| Minor | 1 | Setup instructions reference missing hygiene files. |
| Nit | 0 | |

---

## What's working

- **Subcommand Synced**: Both the README and the User Manual now accurately refer to the `validate-model` and `validate-gcode` CLI subcommands rather than `validate`.
- **Rust Enum Constructor**: Code examples correctly use the enum variant constructor `BuildVolume::Rectangular` rather than constructing the enum as a struct.
- **JSON Format Corrected**: The User Manual now includes the `"type": "rectangular"` tag and uses `"low"` instead of `"easy"` for difficulty ratings in its JSON profile examples.
- **Git Hook Setup**: `CONTRIBUTING.md` now contains the full shell script content for the pre-push hook.

---

## What couldn't be assessed

- **Stage 2 Features**: The MCP server and advanced adapters are mocked or stubbed. While their presence in the CLI/SDK is simulated, their operational behavior remains out of scope for Stage 1.

---

## Doc asset inventory

| Asset | Exists? | Status | Finding(s) |
|---|---|---|---|
| README.md | Yes | Minor issue | DOC-009 |
| ARCHITECTURE.md | Yes | Accurate | None |
| USER_MANUAL.md | Yes | Broken examples | DOC-009 |
| API_REFERENCE.md | Yes | Out of sync / Incomplete | DOC-010, DOC-011 |
| FAQ.md | Yes | Accurate | None |
| CHANGELOG.md | Yes | Accurate | None |
| CONTRIBUTING.md | Yes | Accurate | None |
| SECURITY / LICENSE | No | Absent locally | DOC-008 |

---

## Persona walk-through

### First-time user
- **Onboarding blocker**: A user attempting to verify the installation of PrintProof3D by copying the quickstart CLI command gets a file-not-found error because the referenced `profiles/` directory and JSON files are not included in the repository.
- **Result**: The first-time user cannot run the validation tool with the example profiles.

### Returning user
- **Navigation blocker**: A developer looking at `API_REFERENCE.md` tries to look up the fields of `PrinterProfile` or the structure of `PrinterTelemetry` but finds them missing from the reference list, requiring them to look at the Rust source code.
- **Result**: Developer friction and reduced trust in documentation.

### New team member
- **Developer onboarding**: A contributor can successfully set up the git hook using the script provided in `CONTRIBUTING.md`.
- **Result**: Smooth developer setup.

---

## Verification of Previous Findings

| Finding ID | Previous Severity | Title | Status | Comments |
|---|---|---|---|---|
| **DOC-001** | Blocker | Out-of-Sync CLI Subcommands | **Resolved** | Updated to `validate-model` and `validate-gcode`. |
| **DOC-002** | Blocker | Compile-Breaking Rust Code Examples | **Resolved** | Changed to `BuildVolume::Rectangular` variant. |
| **DOC-003** | Blocker | JSON Printer Profile Parse Failures | **Resolved** | Added `"type": "rectangular"` tag to example JSON. |
| **DOC-004** | Blocker | JSON Material Profile Value Errors | **Resolved** | Replaced `"easy"` with `"low"` risk level string. |
| **DOC-005** | Major | Struct Property Name/Type Drift | **Resolved** | Types synced to `RiskLevel` and missing properties documented. |
| **DOC-006** | Major | Omission of Core Public Traits | **Partially Resolved** | Core traits (`ModelValidator`, `GcodeValidator`, `PrinterAdapter`) are now documented, but related structs/enums are still missing (see DOC-011). |
| **DOC-007** | Minor | Untracked Git Hook Instructions | **Resolved** | The pre-push hook script content is now written in `CONTRIBUTING.md`. |
| **DOC-008** | Minor | Missing Local LICENSE and SECURITY.md | **Unresolved** | Direct files are still absent in the `PrintProof3D` directory. |

---

## New Findings

### [DOC-009] — Blocker — Onboarding — Missing Example Profile JSON Files

**Evidence**
- **Files**: `README.md` (lines 57, 60), `USER_MANUAL.md` (lines 80-83, 110-112)
- **Problem**: The CLI validation commands in the documentation use profile JSON paths like `profiles/prusa_mk4.json` and `profiles/pla.json`. However, no `profiles/` directory or files are included in the workspace repository.
- **Why this matters**: First-time users trying to run the quickstart commands will fail with a file-not-found error, preventing them from testing the application's CLI behavior.
- **Blast radius**: `README.md` and `USER_MANUAL.md` CLI examples.
- **Fix path**:
  Create a directory `profiles/` in the workspace root, and add:
  1. `profiles/prusa_mk4.json` with the JSON content shown in the User Manual (under *Defining a Printer Profile*).
  2. `profiles/pla.json` with the JSON content shown in the User Manual (under *Defining a Material Profile*).

---

### [DOC-010] — Major — API — Documented Fields Drift in API Reference

**Evidence**
- **File**: `API_REFERENCE.md` (lines 11-25)
- **Problem**: The listed fields for the `PrinterProfile` struct in the API Reference omit several connectivity-related and quirk-related fields. Specifically, the following fields from `crates/core/src/lib.rs` are missing from the list:
  - `supports_direct_upload: bool`
  - `supports_pause_resume: bool`
  - `supports_cancel: bool`
  - `supports_job_progress: bool`
  - `supports_webcam: bool`
  - `supports_chamber_temp: bool`
  - `known_quirks: Vec<String>`
  - `unsafe_commands: Vec<String>`
  - `filename_restrictions: Option<String>`
- **Why this matters**: A library developer relying on the list in the API reference would believe these fields do not exist or are not exposed, creating confusion since they are required to instantiate the struct (as shown in the code example).
- **Blast radius**: `API_REFERENCE.md` reference table/list.
- **Fix path**:
  Update `API_REFERENCE.md` lines 11-26 to list all 24 fields.

---

### [DOC-011] — Major — API — Omission of Public Adapter Helper Types

**Evidence**
- **File**: `API_REFERENCE.md` (line 111)
- **Problem**: The `PrinterAdapter` trait methods return `Result<PrinterTelemetry, AdapterError>`. While `PrinterAdapter` is documented, the definitions of the structures/enums it depends on (`PrinterTelemetry` and `AdapterError`) are omitted from `API_REFERENCE.md`, despite being public models exported by `crates/adapters/src/lib.rs`.
- **Why this matters**: Integrators looking to implement their own adapters or inspect status telemetry from physical printer adapters cannot find the fields or enum variants without digging into the source code.
- **Blast radius**: `API_REFERENCE.md` adapter section.
- **Fix path**:
  Add sections documenting `PrinterTelemetry` and `AdapterError` under the `printproof3d-adapters` section in `API_REFERENCE.md`.

---

## Suggested Rewrites

Below are the drafted replacements for the affected documentation parts to address the new findings:

### 1. Updated `API_REFERENCE.md` (Crate models & adapter details)

```markdown
# PrintProof3D Rust API Reference

This reference documents the public structures, traits, and functions exposed by the PrintProof3D crates.

## 1. `printproof3d-core`

### Core Structs

#### `PrinterProfile`
Defines printer limits and capacities.
- `manufacturer: String` — The manufacturer name (e.g. "Prusa").
- `model: String` — The specific model name (e.g. "MK4").
- `protocol_family: ProtocolFamily` — The communication protocol standard.
- `build_volume: BuildVolume` — Bounding box limits in mm.
- `bed_shape: BedShape` — Visual shape layout of the build plate.
- `nozzle_diameters: Vec<f32>` — Supported nozzle diameters in mm.
- `default_nozzle_diameter: f32` — Default nozzle diameter in mm.
- `min_layer_height: f32` — Minimum layer height in mm.
- `max_layer_height: f32` — Maximum layer height in mm.
- `max_hotend_temp: f32` — Maximum safe hotend temperature in Celsius.
- `max_bed_temp: f32` — Maximum safe bed temperature in Celsius.
- `has_enclosure: bool` — Chamber enclosure indicator.
- `supports_mmu: bool` — Automatic material unit support indicator.
- `firmware_flavor: FirmwareFlavor` — Internal firmware flavor.
- `supported_file_types: Vec<String>` — Extensions supported (e.g., ["gcode"]).
- `supports_direct_upload: bool` — Direct remote print upload support.
- `supports_pause_resume: bool` — Pause/resume state control support.
- `supports_cancel: bool` — Active job cancellation support.
- `supports_job_progress: bool` — Live progress telemetry support.
- `supports_webcam: bool` — Webcam stream support.
- `supports_chamber_temp: bool` — Chamber temp monitoring support.
- `known_quirks: Vec<String>` — Bug overrides list.
- `unsafe_commands: Vec<String>` — Slicer blacklisted G-code instructions.
- `filename_restrictions: Option<String>` — Regex pattern for uploaded filenames.

#### `MaterialProfile`
Defines thermal thresholds and characteristics of printing filament.
- `name: String`
- `abbreviations: Vec<String>`
- `min_nozzle_temp: f32`
- `max_nozzle_temp: f32`
- `min_bed_temp: f32`
- `max_bed_temp: f32`
- `cooling_fan_speed_pct: f32`
- `warp_risk: RiskLevel`
- `bridge_difficulty: RiskLevel`
- `overhang_difficulty: RiskLevel`
- `enclosure_recommended: bool`
- `dryness_sensitive: bool`
- `bed_adhesion_notes: Option<String>`
- `min_feature_size_mm: f32`

#### `ValidationReport`
The output of validation engines.
- `status: ValidationStatus`
- `target_printer_profile: String`
- `target_material_profile: String`
- `model: ModelMetadata`
- `issues: Vec<ValidationIssue>`
- `confidence_level: String`
- `sliced_settings_assumed: Option<serde_json::Value>`

---

### Core Enums

#### `BuildVolume`
Represents the dimensional printing bounds.
```rust
pub enum BuildVolume {
    Rectangular { x: f32, y: f32, z: f32 },
    Cylindrical { diameter: f32, z: f32 },
}
\```

#### `RiskLevel`
Represents warning risk layers.
```rust
pub enum RiskLevel {
    Low,
    Medium,
    High,
}
\```

---

## 2. `printproof3d-printability`

### Public Traits

#### `ModelValidator`
Validates geometric properties of 3D models.
```rust
pub trait ModelValidator {
    fn validate_mesh(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}
\```

#### `GcodeValidator`
Statically audits path bounds and firmware compatibility of G-code files.
```rust
pub trait GcodeValidator {
    fn validate_gcode(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}
\```

---

## 3. `printproof3d-adapters`

### Public Traits & Telemetry

#### `PrinterAdapter`
Defines communication capabilities with physical printer nodes.
```rust
#[async_trait]
pub trait PrinterAdapter: Send + Sync {
    async fn connect(&mut self) -> Result<(), AdapterError>;
    async fn disconnect(&mut self) -> Result<(), AdapterError>;
    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError>;
    async fn upload_file(&self, local_path: &Path, remote_name: &str) -> Result<String, AdapterError>;
    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError>;
    async fn pause_job(&self) -> Result<(), AdapterError>;
    async fn resume_job(&self) -> Result<(), AdapterError>;
    async fn cancel_job(&self) -> Result<(), AdapterError>;
    async fn emergency_stop(&self) -> Result<(), AdapterError>;
}
\```

#### `PrinterTelemetry`
Contains state snapshots retrieved from the print adapter.
- `state: String` — Active status ("idle", "printing", "paused", "error").
- `tool_temp: f32` — Current extruder hotend temperature in Celsius.
- `tool_target: f32` — Target extruder hotend temperature in Celsius.
- `bed_temp: f32` — Current heated bed temperature in Celsius.
- `bed_target: f32` — Target heated bed temperature in Celsius.
- `progress: f32` — Print progress percentage (0.0 to 100.0).
- `current_file: Option<String>` — Active printing file name.

#### `AdapterError`
Enumerate connection and control command failure states.
```rust
pub enum AdapterError {
    ConnectionFailed(String),
    AuthenticationFailed(String),
    UploadFailed(String),
    CommandFailed(String),
    Timeout,
    Unknown(String),
}
\```

---

## Code Example

```rust
use printproof3d_core::{PrinterProfile, BuildVolume, BedShape, ProtocolFamily, FirmwareFlavor};
use printproof3d_sdk::sdk_init;

fn main() {
    // Initialize SDK
    sdk_init();

    // Define configuration
    let printer = PrinterProfile {
        manufacturer: "Custom".to_string(),
        model: "Prusa Clone".to_string(),
        protocol_family: ProtocolFamily::MarlinSerial,
        build_volume: BuildVolume::Rectangular { x: 220.0, y: 220.0, z: 250.0 },
        bed_shape: BedShape::Rectangular,
        nozzle_diameters: vec![0.4],
        default_nozzle_diameter: 0.4,
        min_layer_height: 0.1,
        max_layer_height: 0.3,
        max_hotend_temp: 260.0,
        max_bed_temp: 100.0,
        has_enclosure: false,
        supports_mmu: false,
        firmware_flavor: FirmwareFlavor::Marlin,
        supported_file_types: vec!["gcode".to_string()],
        supports_direct_upload: true,
        supports_pause_resume: false,
        supports_cancel: true,
        supports_job_progress: false,
        supports_webcam: false,
        supports_chamber_temp: false,
        known_quirks: vec![],
        unsafe_commands: vec![],
        filename_restrictions: None,
    };

    println!("Loaded profile: {} {}", printer.manufacturer, printer.model);
}
\```
```

---

## Drafts produced

The following drafts have been updated with complete corrections:
- `doc-rewrites/README.md`
- `doc-rewrites/ARCHITECTURE.md`
- `doc-rewrites/USER_MANUAL.md`
- `doc-rewrites/API_REFERENCE.md`
- `doc-rewrites/CONTRIBUTING.md`

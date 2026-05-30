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
```

#### `RiskLevel`
Represents warning risk layers.
```rust
pub enum RiskLevel {
    Low,
    Medium,
    High,
}
```

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
```

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
```

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
```

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
```

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
```

# PrintProof3D Rust API Reference

This reference documents the public structures, traits, and functions exposed by the PrintProof3D crates.

---

## 1. `printproof3d-core`

### Core Structs

#### `PrinterProfile`
Defines printer limits and capacities.
- `manufacturer: String` — The manufacturer name.
- `model: String` — The specific model name.
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

---

## 4. `printproof3d-sdk`

### Conformance Testing

#### `run_conformance_tests`
Automated validation checks to verify `PrinterAdapter` implementation compliance.
```rust
pub async fn run_conformance_tests<A: PrinterAdapter>(adapter: &mut A) -> Result<(), String>;
```

---

## 5. `printproof3d-plugins`

### WASM Loader Runtime

#### `PluginEngine`
Initializes compiler-free WebAssembly engines.
- `pub fn new() -> Self` — Creates a new sandbox configuration.
- `pub fn load_plugin(&self, wasm_bytes: &[u8]) -> Result<LoadedPlugin, String>` — Instantiates a WASM module.

#### `LoadedPlugin`
Executes loaded plugins in the wasmi context.
- `pub fn execute_validation(&mut self, report_json: &str) -> Result<String, String>` — Pass a JSON report representation to the guest and return the modified result.

### Helper Macros

#### `export_validation_plugin!`
Simplifies memory mapping and serialization in custom WASM modules:
```rust
export_validation_plugin!(validation_function);
```

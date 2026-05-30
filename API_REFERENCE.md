# PrintProof3D: Core API Reference Manual

This document is the canonical developer API reference manual for the PrintProof3D crates. It details public data structures, validation traits, communication protocols, memory allocation functions, and plugin sandbox APIs.

---

## 1. `printproof3d-core` — Shared Models & Schemas

The `printproof3d-core` crate houses the shared data structures and configuration profiles. All configurations enforce strict validation invariants on initialization.

### 1.1 `PrinterProfile` (Struct)
Defines the physical boundaries, capacities, kinematics, and communication capabilities of the machine.

#### Fields:
* **`manufacturer: String`**: The manufacturer name (e.g. `"Prusa"`). Must be non-empty.
* **`model: String`**: The specific model name (e.g. `"MK4"`). Must be non-empty.
* **`protocol_family: ProtocolFamily`**: Communication protocol enum used to determine the adapter type. Valid options:
  * `Klipper`, `OctoPrint`, `MarlinSerial`, `PrusaLink`, `RepRapFirmware`, `BambuMqtt`, `ElegooSdcp`, `CrealityOs`, `AnycubicLan`, `FlashForgeTcp`, `Unknown`.
* **`build_volume: BuildVolume`**: Represents the physical limits of the print volume.
  * `BuildVolume::Rectangular { x: f32, y: f32, z: f32 }`: Length, width, and height bounds. All dimensions must be $> 0.0$.
  * `BuildVolume::Cylindrical { diameter: f32, z: f32 }`: Diameter and height limits. All dimensions must be $> 0.0$.
* **`bed_shape: BedShape`**: Physical bed layout: `Rectangular`, `Circular`, or `Custom(String)`.
* **`nozzle_diameters: Vec<f32>`**: List of nozzle diameters supported (in mm). Every value must be $> 0.0$.
* **`default_nozzle_diameter: f32`**: Nozzle size loaded on the machine by default. Must exist within `nozzle_diameters`.
* **`min_layer_height: f32`**: Minimal practical layer thickness. Must be $> 0.0$.
* **`max_layer_height: f32`**: Maximum practical layer thickness. Must be $\ge \text{min\_layer\_height}$.
* **`max_hotend_temp: f32`**: Physical heating limit of the hotend. Must be positive and $\le 500^\circ\text{C}$ for safety.
* **`max_bed_temp: f32`**: Physical heating limit of the bed. Must be positive and $\le 200^\circ\text{C}$ for safety.
* **`has_enclosure: bool`**: Indicates if the machine is fully enclosed.
* **`supports_mmu: bool`**: Indicates multi-material capabilities.
* **`firmware_flavor: FirmwareFlavor`**: Firmware style: `Klipper`, `Marlin`, `RepRapFirmware`, `Prusa`, `Bambu`, `Elegoo`, `Creality`, `Anycubic`, `FlashForge`, `Unknown`.
* **`supported_file_types: Vec<String>`**: File extensions accepted (e.g. `["gcode", "bgcode"]`).
* **`supports_direct_upload: bool`**: Indicates network file upload support.
* **`supports_pause_resume: bool`**: Indicates pause/resume state control support.
* **`supports_cancel: bool`**: Indicates cancellation support.
* **`supports_job_progress: bool`**: Indicates progress telemetry reporting.
* **`supports_webcam: bool`**: Indicates webcam streaming support.
* **`supports_chamber_temp: bool`**: Indicates ambient chamber temp sensors.
* **`known_quirks: Vec<String>`**: Known driver or controller bugs to bypass.
* **`unsafe_commands: Vec<String>`**: Blacklisted G-code instructions.
* **`filename_restrictions: Option<String>`**: Optional regular expression pattern to validate file names.

#### Key Invariants & Validations:
* `PrinterProfile::validate(&self) -> Result<(), String>`:
  * Ensures strings are not empty and dimensions are positive.
  * Verifies compatibility between bed shape and build volume: `Circular` beds require `Cylindrical` volumes, and `Rectangular` beds require `Rectangular` volumes.
  * Rejects thermal thresholds above physical safety envelopes (hotend $> 500^\circ\text{C}$, bed $> 200^\circ\text{C}$).

---

### 1.2 `MaterialProfile` (Struct)
Defines the filament properties and thermal processing ranges.

#### Fields:
* **`name: String`**: Chemical or branding name (e.g. `"Polylactic Acid"`). Must be non-empty.
* **`abbreviations: Vec<String>`**: Abbreviated identifiers (e.g., `["PLA", "PLA+"]`).
* **`min_nozzle_temp: f32`**: Minimum extrusion temperature in Celsius. Must be $> 0.0$.
* **`max_nozzle_temp: f32`**: Maximum extrusion temperature in Celsius. Must be $\ge \text{min\_nozzle\_temp}$.
* **`min_bed_temp: f32`**: Minimum bed adhesion temperature in Celsius. Must be $> 0.0$.
* **`max_bed_temp: f32`**: Maximum bed adhesion temperature in Celsius. Must be $\ge \text{min\_bed\_temp}$.
* **`cooling_fan_speed_pct: f32`**: Target extruder cooling fan speed (from `0.0` to `100.0`).
* **`warp_risk: RiskLevel`**: Relative warping risk: `Low`, `Medium`, or `High`.
* **`bridge_difficulty: RiskLevel`**: Difficulty bridging horizontal spans: `Low`, `Medium`, or `High`.
* **`overhang_difficulty: RiskLevel`**: Difficulty cooling steep overhang slopes: `Low`, `Medium`, or `High`.
* **`enclosure_recommended: bool`**: Indicates if printing requires an enclosed build chamber.
* **`dryness_sensitive: bool`**: Indicates if the material is hygroscopic and needs drying.
* **`bed_adhesion_notes: Option<String>`**: Optional notes on bed prep.
* **`min_feature_size_mm: f32`**: Smallest printable detailed width. Must be $> 0.0$.

---

### 1.3 `ValidationReport` (Struct)
The unified report structure returned by validation passes.

#### Fields:
* **`status: ValidationStatus`**: Consolidation of safety checks.
  * `ValidationStatus::Pass`: The file matches all profiles and is safe to print.
  * `ValidationStatus::Warning`: The file contains non-blocking warnings (e.g. small contact footprint).
  * `ValidationStatus::Fail`: The file violates physical safety boundaries (e.g. out of bounds or excessive temperatures).
* **`target_printer_profile: String`**: Concatened manufacturer and model used during validation.
* **`target_material_profile: String`**: Material name used during validation.
* **`model: ModelMetadata`**: Bounding box size and filename metadata.
* **`issues: Vec<ValidationIssue>`**: List of compatibility warnings or failures.
* **`confidence_level: String`**: Analysis confidence ranking: `"high"`, `"medium"`, or `"low"`.
* **`sliced_settings_assumed: Option<serde_json::Value>`**: Optional key-value storage for slicing parameters.

#### Key Invariants & Validations:
* `ValidationReport::validate(&self) -> Result<(), String>`:
  * Enforces report integrity: if the report contains any `Critical` or `Blocker` severity issues, the `status` **must** be set to `ValidationStatus::Fail`.

---

## 2. `printproof3d-printability` — Verification Engines

This crate provides the core validation traits and algorithms for mesh parsing and static path analysis.

### 2.1 `ModelValidator` (Trait)
Exposes the entry point for 3D model geometry analysis.

```rust
pub trait ModelValidator {
    /// Parses an STL file and validates mesh watertightness, 
    /// build volume limits, overhang angles, and bed contact area.
    fn validate_mesh(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}
```
* **Implementations**: `StlModelValidator` parses ASCII STL files, processes face normal vector orientations, and performs edge counting.

---

### 2.2 `GcodeValidator` (Trait)
Exposes the entry point for G-code static parsing.

```rust
pub trait GcodeValidator {
    /// Scans a G-code file line-by-line, tracking kinematic positions 
    /// and temperature targets to ensure they conform to printer boundaries.
    fn validate_gcode(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}
```
* **Implementations**: `StandardGcodeValidator` parses coordinates, handles homing (`G28`), tracks movement mode changes (`G90`/`G91`), and monitors thermal commands (`M104`, `M109`, `M140`, `M190`).

---

## 3. `printproof3d-adapters` — Communication Protocols

This crate standardizes communication interfaces across printer connection protocols.

### 3.1 `PrinterAdapter` (Trait)
Third-party clients implement this asynchronous trait to enable remote control and telemetry querying.

```rust
#[async_trait]
pub trait PrinterAdapter: Send + Sync {
    /// Opens the communication socket or serial channel.
    async fn connect(&mut self) -> Result<(), AdapterError>;
    
    /// Gracefully closes connection streams.
    async fn disconnect(&mut self) -> Result<(), AdapterError>;
    
    /// Polls status parameters and active temperatures.
    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError>;
    
    /// Uploads a print file to the printer's local storage.
    async fn upload_file(
        &self, 
        local_path: &Path, 
        remote_name: &str
    ) -> Result<String, AdapterError>;
    
    /// Triggers job execution.
    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError>;
    
    /// Pauses execution.
    async fn pause_job(&self) -> Result<(), AdapterError>;
    
    /// Resumes execution.
    async fn resume_job(&self) -> Result<(), AdapterError>;
    
    /// Cancels execution.
    async fn cancel_job(&self) -> Result<(), AdapterError>;
    
    /// Halts motion and disables power immediately.
    async fn emergency_stop(&self) -> Result<(), AdapterError>;
}
```

### 3.2 Error & Telemetry Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdapterError {
    ConnectionFailed(String),
    AuthenticationFailed(String),
    UploadFailed(String),
    CommandFailed(String),
    Timeout,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrinterTelemetry {
    pub state: String,
    pub tool_temp: f32,
    pub tool_target: f32,
    pub bed_temp: f32,
    pub bed_target: f32,
    pub progress: f32,
    pub current_file: Option<String>,
}
```

---

## 4. `printproof3d-sdk` — Developer Compliance SDK

Exposes automated test verification harnesses to validate connection adapter trait implementations.

### 4.1 Conformance Runner
Runs a suite of automated state machine checks, verifying connection state transitions and state reporting.

```rust
/// Exercises connection handshake, status polling, and job pause/abort loops.
pub async fn run_conformance_tests<A: PrinterAdapter>(adapter: &mut A) -> Result<(), String>;
```

---

## 5. `printproof3d-plugins` — WebAssembly Runtime

Provides the linear memory management interfaces and interpreter hooks to execute validation plugins.

### 5.1 `PluginEngine` (Struct)
Compiles and instantiates guest WASM modules.
* `pub fn new() -> Self`: Initializes the underlying WASM compilation engine.
* `pub fn load_plugin(&self, wasm_bytes: &[u8]) -> Result<LoadedPlugin, String>`: Compiles a raw bytecode slice, linking standard allocation stubs and returning an executable plugin.

### 5.2 `LoadedPlugin` (Struct)
Manages the memory boundary and executes validation functions.
* `pub fn execute_validation(&mut self, report_json: &str) -> Result<String, String>`:
  1. Calls `alloc` on the guest to reserve memory.
  2. Writes the input report JSON string into the guest's linear memory.
  3. Executes the guest's `validate` function.
  4. Reads the result JSON bytes from the guest's memory.
  5. Cleans up guest allocations via `dealloc`.

### 5.3 Export Macro (`export_validation_plugin!`)
Developer macro to export required guest symbols (`alloc`, `dealloc`, `validate`):

```rust
#[macro_export]
macro_rules! export_validation_plugin {
    ($validate_fn:expr) => {
        #[no_mangle]
        pub extern "C" fn alloc(size: u32) -> *mut u8 { ... }

        #[no_mangle]
        pub extern "C" fn dealloc(ptr: *mut u8, size: u32) { ... }

        #[no_mangle]
        pub extern "C" fn validate(ptr: *mut u8, len: u32) -> u64 { ... }
    }
}
```
* **Memory Management**: Allocation uses `std::alloc::alloc` and `std::alloc::dealloc` with an alignment of 8 bytes.
* **Return Packaging**: Packs the 32-bit output pointer and 32-bit output length into a single `u64`:
  `((output_ptr as u64) << 32) | (output_len as u64)`

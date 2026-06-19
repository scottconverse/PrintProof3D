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
* **`protocol_family: ProtocolFamily`**: Communication protocol enum used to determine the adapter type. Valid options (serialized as snake_case strings):
  * `klipper`, `octo_print`, `marlin_serial`, `prusa_link`, `rep_rap_firmware`, `bambu_mqtt` (fully supported).
  * `elegoo_sdcp`, `creality_os`, `anycubic_lan`, `flash_forge_tcp` (stubbed placeholders; choosing these will trigger validation errors on connection profile initialization until active support is added).
  * `unknown`.
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
* **`firmware_flavor: FirmwareFlavor`**: Internal firmware parser flavor (`rep_rap`, `klipper`, `marlin`, `bambu`, `prusa`).
* **`supported_file_types: Vec<String>`**: List of file extensions supported for direct execution (e.g. `["gcode"]`).
* **`supports_direct_upload: bool`**: Indicates direct remote print upload connectivity support.
* **`supports_pause_resume: bool`**: Indicates job pause and resume state control support.
* **`supports_cancel: bool`**: Indicates active job cancellation support.
* **`supports_job_progress: bool`**: Indicates live print percentage and telemetry reporting support.
* **`supports_webcam: bool`**: Indicates webcam remote monitoring streaming availability.
* **`supports_chamber_temp: bool`**: Indicates active chamber temperature monitoring availability.
* **`known_quirks: Vec<String>`**: Known configuration or driver bugs to bypass.
* **`unsafe_commands: Vec<String>`**: Slicer blacklisted G-code instructions.
* **`filename_restrictions: Option<String>`**: Target file name constraints regular expression pattern.

---

### 1.2 `MaterialProfile` (Struct)
Defines the chemical limits, temperature envelopes, and printing characteristics of a target filament.

#### Fields:
* **`name: String`**: Readable name of the material (e.g. `"Polylactic Acid"`). Must be non-empty.
* **`abbreviations: Vec<String>`**: Known abbreviations (e.g. `["PLA", "PLA+"]`).
* **`min_nozzle_temp: f32`**: Minimum recommended nozzle temperature in Celsius. Must be positive.
* **`max_nozzle_temp: f32`**: Maximum recommended nozzle temperature in Celsius. Must be $\ge \text{min\_nozzle\_temp}$.
* **`min_bed_temp: f32`**: Minimum recommended bed temperature in Celsius. Must be non-negative.
* **`max_bed_temp: f32`**: Maximum recommended bed temperature in Celsius. Must be $\ge \text{min\_bed\_temp}$.
* **`cooling_fan_speed_pct: f32`**: Extruder cooling fan speed percentage (0.0 to 100.0).
* **`warp_risk: RiskLevel`**: Relative warp risk under standard airflow conditions (`Low`, `Medium`, or `High`).
* **`bridge_difficulty: RiskLevel`**: Bridge print extrusion difficulty (`Low`, `Medium`, or `High`).
* **`overhang_difficulty: RiskLevel`**: Overhang print angle cooling difficulty (`Low`, `Medium`, or `High`).
* **`enclosure_recommended: bool`**: True if an enclosure is recommended for this material.
* **`dryness_sensitive: bool`**: True if the raw material absorbs ambient moisture easily (hygroscopic).
* **`bed_adhesion_notes: Option<String>`**: Help descriptions for bed preparation.
* **`min_feature_size_mm: f32`**: Smallest resolvable detailed dimension in mm.

---

### 1.3 `ValidationReport` (Struct)
The unified serialization payload returned by all printability validation runs.

#### Fields:
* **`status: ValidationStatus`**: Outcome classification (`Pass`, `Warning`, `Fail`).
* **`target_printer_profile: String`**: Name of the printer profile evaluated.
* **`target_material_profile: String`**: Name of the material profile evaluated.
* **`model: Option<ModelMetadata>`**: Geometry metadata (bounding box, size) of evaluated STL.
* **`issues: Vec<ValidationIssue>`**: Array of parsed warnings and errors.
* **`confidence_level: String`**: Reliability confidence indicator (`high` or `medium`).
* **`sliced_settings_assumed: Option<HashMap<String, serde_json::Value>>`**: Decoded parameters extracted from G-code or simulation runs (e.g. `simulator_telemetry`).

#### Key Invariants & Validations:
* Enforces report integrity: if the report contains any `Critical` or `Blocker` severity issues, the `status` **must** be set to `ValidationStatus::Fail`.

---

### 1.4 `PrinterConnectionConfig` (Struct)
Defines the connection parameters, credentials, endpoints, and pre-flight execution policies required to communicate with a remote printer or simulator.

#### Fields:
* **`name: String`**: A descriptive human-readable label for the target. Must be non-empty.
* **`mode: ConnectionMode`**: Indicates whether to route to a simulation host or physical hardware. Valid options (serialized as snake_case strings): `simulator`, `physical`.
* **`protocol_family: ProtocolFamily`**: Network/communication protocol enum.
* **`base_url: Option<String>`**: Base URL or IP address. Required for physical network protocol targets.
* **`serial_path: Option<String>`**: Serial port device endpoint path. Required for physical MarlinSerial connections.
* **`serial_baud_rate: Option<u32>`**: Serial connection baud rate.
* **`auth_type: AuthType`**: Authentication mechanism. Valid options (serialized as snake_case strings): `none`, `api_key`, `digest`, `password`.
* **`api_key_env_var: Option<String>`**: Name of the environment variable storing the API key. Required for `AuthType::ApiKey`.
* **`username: Option<String>`**: Username for authentication. Required for `AuthType::Password`.
* **`password_env_var: Option<String>`**: Name of the environment variable storing the password. Required for `AuthType::Password`.
* **`tls_enabled: bool`**: Activates secure socket TLS.
* **`dispatch_policy: DispatchPolicy`**: Pre-flight action permission rules. Valid options (serialized as snake_case strings): `dry_run_only`, `upload_only`, `allow_start`.
* **`simulator_scenario: Option<SimulatorScenario>`**: Optional scenario to load when `mode` is `simulator`.

#### Key Invariants & Validations:
* `PrinterConnectionConfig::validate(&self) -> Result<(), String>`:
  * Rejects empty target names.
  * Rejects unsupported placeholder protocol variants (`elegoo_sdcp`, `creality_os`, `anycubic_lan`, `flash_forge_tcp`).
  * For physical mode, checks that `serial_path` is set if protocol is `MarlinSerial`.
  * For physical mode, checks that `base_url` is set if protocol is network-based.
  * For API Key auth, ensures `api_key_env_var` is specified.
  * For Password auth, ensures both `username` and `password_env_var` are specified.

---

### 1.5 `SimulatorScenario` (Enum)
The set of standard simulation scenarios used to dry-run and QA printer state transitions inside simulated adapters (serialized as snake_case strings):
* **`idle`**: Mock server reports idle state.
* **`already_printing`**: Mock server reports active print, blocking new uploads.
* **`paused`**: Mock server reports print paused state.
* **`heating`**: Mock server simulates heater temperature climb state.
* **`upload_accepted`**: Mock server accepts and registers the uploaded print job.
* **`upload_rejected`**: Mock server rejects file uploads.
* **`bad_credentials`**: Mock server returns access denied / authentication error.
* **`offline_or_connection_refused`**: Simulates socket refuse / network down.
* **`timeout_or_slow_response`**: Simulates network timeout.
* **`malformed_telemetry`**: Simulates corrupt status packet return.
* **`storage_full`**: Simulates insufficient flash/disk storage.
* **`unsupported_file_type`**: Simulates file extension rejection.
* **`emergency_stop_accepted`**: Simulates emergency halt accepted.
* **`emergency_stop_rejected`**: Simulates emergency halt rejected.

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
    pub state: PrinterState,
    pub tool_temp: f32,
    pub tool_target: f32,
    pub bed_temp: f32,
    pub bed_target: f32,
    pub progress: f32,
    pub current_file: Option<String>,
}
```

---

### 3.3 `PrinterAdapterFactory` (Struct)
Central registry factory to build dynamic adapter instances from profiles and configurations.

```rust
pub struct PrinterAdapterFactory;

impl PrinterAdapterFactory {
    /// Validates the connection config and returns an initialized dynamic box adapter matching the target protocol.
    pub fn build(
        profile: &PrinterProfile,
        config: &PrinterConnectionConfig,
    ) -> Result<Box<dyn PrinterAdapter>, AdapterError>;
}
```

---

### 3.4 Telemetry Enums
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrinterState {
    Idle,
    Printing,
    Paused,
    Error,
    Unknown,
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

---

## 6. `printproof3d` Command-Line Interface (CLI) & JSON Serialization Contracts

The CLI executable integrates the core modules and exposes profile discovery, schema validation, and multi-dimensional compatibility check capabilities.

### 6.1 Discover Profiles
Discover valid JSON profiles located in a given directory path.

#### Commands:
* **`list-printers`**: Scans a directory for printer profiles.
* **`list-materials`**: Scans a directory for material profiles.

#### Arguments & Options:
* `-d, --directory <PATH>`: Custom directory path to scan. Defaults to `profiles/`.
* `-f, --format <text|json>`: Output presentation structure. Defaults to `text`.

#### Examples:
```bash
# Print printers text list
target/release/printproof3d.exe list-printers --format text

# Print materials JSON structure (alphabetically sorted by name)
target/release/printproof3d.exe list-materials --format json
```

### 6.2 Profile Inspection
Read, auto-detect profile structure, and display decoded parameters.

#### Commands:
* **`inspect-profile <FILE>`**: Auto-detects profile type (printer or material) and inspects its internal field structures.

#### Arguments & Options:
* `<FILE>`: Path to JSON file to inspect.
* `-f, --format <text|json>`: Output presentation structure. Defaults to `text`.

#### Examples:
```bash
# Inspect printer profile in human-readable text
target/release/printproof3d.exe inspect-profile profiles/prusa_mk4.json

# Inspect material profile in wrapped JSON
target/release/printproof3d.exe inspect-profile profiles/pla.json --format json
```

### 6.3 Profile Validation
Verify JSON schema structures and enforce safety bounds (e.g. maximum temperatures).

#### Commands:
* **`validate-printer-profile <FILE>`**: Validates printer JSON file structure.
* **`validate-material-profile <FILE>`**: Validates material JSON file structure.

#### Arguments & Options:
* `<FILE>`: Path to JSON file to validate.
* `-f, --format <text|json>`: Output presentation structure. Defaults to `text`.

#### Examples:
```bash
# Validate printer profile
target/release/printproof3d.exe validate-printer-profile profiles/prusa_mk4.json

# Validate material profile (JSON output format)
target/release/printproof3d.exe validate-material-profile profiles/pla.json --format json
```

### 6.4 Compatibility Checks
Audits interactions between target printers, materials, and geometric files (STL models or G-code toolpaths).

#### Commands:
* **`check-compatibility`**: Runs multi-dimensional audits to verify alignment between machine specifications, material limits, and geometric assets.

#### Arguments & Options:
* `-p, --printer <PRINTER_FILE>`: (Required) Target printer profile path.
* `-a, --material <MATERIAL_FILE>`: (Optional) Material profile path.
* `-m, --model <MODEL_FILE>`: (Optional) STL model geometry path.
* `-g, --gcode <GCODE_FILE>`: (Optional) Sliced G-code toolpath.
* `-f, --format <text|json>`: Output presentation structure. Defaults to `text`.

#### Examples:
```bash
# Verify printer + material profile compatibility
target/release/printproof3d.exe check-compatibility --printer profiles/prusa_mk4.json --material profiles/pla.json

# Verify printer + model volume footprint compatibility
target/release/printproof3d.exe check-compatibility --printer profiles/prusa_mk4.json --model fixtures/tetrahedron.stl

# Verify printer + sliced G-code compatibility
target/release/printproof3d.exe check-compatibility --printer profiles/prusa_mk4.json --gcode fixtures/safe_print.gcode
```

### 6.5 Profile Generation Templates
Generate default template configurations for printer hardware and materials.

#### Commands:
* **`generate-printer-profile`**: Generates a default printer template profile. Note: this command always emits JSON and does not accept the `--format` option.
* **`generate-material-profile`**: Generates a default material template profile. Note: this command always emits JSON and does not accept the `--format` option.

#### Arguments & Options:
* `-o, --output <FILE>`: (Optional) Output file path to write template JSON. If omitted, prints to stdout.

### 6.6 Directory Validation
Validate all profile JSON files inside a target directory.

#### Commands:
* **`validate-profile-directory`**: Scans and validates all JSON profiles within the directory.

#### Arguments & Options:
* `<DIRECTORY>`: Target directory path.
* `-f, --format <text|json>`: Output presentation structure. Defaults to `text`.
* `-o, --output <FILE>`: (Optional) Output file path to write validation summary results.

---

## 7. Axum REST API Router Parity

The Axum REST server (binds to port `3000` by default) exposes the following endpoints:

### GET `/`
* **Auth**: None
* **Description**: Home route, serves the interactive browser validation dashboard (HTML). *Note: This is an intentional release behavior change; the endpoint now returns `text/html` instead of a plain text API status indicator.*

### GET `/profiles/printers`
* **Auth**: None
* **Description**: Lists names of available printer profiles.

### GET `/profiles/materials`
* **Auth**: None
* **Description**: Lists details of available material profiles.

### POST `/validate/model`
* **Auth**: Bearer Token
* **Request**: Multipart form data with fields `model` (STL file), `printer` (JSON profile), `material` (JSON profile).
* **Description**: Analyzes mesh geometry against profiles.
* **Headers**:
  * `X-Validation-Status`: Overall status of validation (`pass`, `warning`, or `fail`).
* **Status Codes**:
  * `200 OK`: Validation completed successfully (returns validation report).
  * `400 Bad Request`: Missing fields or malformed file/profile payloads.
  * `401 Unauthorized`: API token is missing or incorrect.
  * `500 Internal Server Error`: Validation thread panicked.
* **Response Shape (200 OK)**:
  ```json
  {
    "status": "pass",
    "target_printer_profile": "Prusa_MK4",
    "target_material_profile": "PLA",
    "model": {
      "file_name": "tetrahedron.stl",
      "units": "mm",
      "bounding_box": {
        "min_x": -5.0,
        "min_y": -5.0,
        "min_z": 0.0,
        "max_x": 5.0,
        "max_y": 5.0,
        "max_z": 8.66
      }
    },
    "issues": [],
    "confidence_level": "high"
  }
  ```
* **Error Payload (400 / 401 / 500)**:
  ```json
  {
    "error": "Error description message"
  }
  ```

### POST `/validate/gcode`
* **Auth**: Bearer Token
* **Request**: Multipart form data with fields `gcode` (G-code file), `printer` (JSON profile), and optional `material` (JSON profile).
* **Description**: Analyzes toolpath kinematics and temperature targets.
* **Headers**:
  * `X-Validation-Status`: Overall status of validation (`pass`, `warning`, or `fail`).
* **Status Codes**:
  * `200 OK`: Validation completed successfully (returns validation report).
  * `400 Bad Request`: Missing fields or malformed file/profile payloads.
  * `401 Unauthorized`: API token is missing or incorrect.
  * `500 Internal Server Error`: Validation thread panicked.
* **Response Shape (200 OK)**:
  ```json
  {
    "status": "pass",
    "target_printer_profile": "Prusa_MK4",
    "target_material_profile": "PLA",
    "model": null,
    "issues": [],
    "confidence_level": "high"
  }
  ```
* **Error Payload (400 / 401 / 500)**:
  ```json
  {
    "error": "Error description message"
  }
  ```

### POST `/profiles/inspect`
* **Auth**: Bearer Token
* **Request**: Multipart form data with field `profile` (JSON profile file).
* **Description**: Decodes and inspects printer or material profile.
* **Status Codes**:
  * `200 OK`: Inspection successful.
  * `400 Bad Request`: Malformed JSON or missing parameter.
  * `401 Unauthorized`: API token is missing or incorrect.

### POST `/profiles/validate/printer`
* **Auth**: Bearer Token
* **Request**: Multipart form data with field `printer` (JSON profile file).
* **Description**: Validates printer profile against structural schemas.
* **Status Codes**:
  * `200 OK`: Validation run complete (response indicates validity).
  * `400 Bad Request`: Missing file field.
  * `401 Unauthorized`: API token is missing or incorrect.

### POST `/profiles/validate/material`
* **Auth**: Bearer Token
* **Request**: Multipart form data with field `material` (JSON profile file).
* **Description**: Validates material profile against structural schemas.
* **Status Codes**:
  * `200 OK`: Validation run complete (response indicates validity).
  * `400 Bad Request`: Missing file field.
  * `401 Unauthorized`: API token is missing or incorrect.

### POST `/validate/compatibility`
* **Auth**: Bearer Token
* **Request**: Multipart form data with fields `printer` (JSON profile, required), and optional `material` (JSON profile), `model` (STL file), and `gcode` (G-code file).
* **Description**: Audits multi-dimensional alignment and returns status (`pass`, `warning`, `fail`).
* **Headers**:
  * `X-Validation-Status`: Overall status of validation (`pass`, `warning`, or `fail`).
* **Status Codes**:
  * `200 OK`: Audit completed successfully (returns validation report).
  * `400 Bad Request`: Missing required `printer` profile, or invalid file payloads.
  * `401 Unauthorized`: API token is missing or incorrect.
  * `500 Internal Server Error`: Validation thread panicked.
* **Response Shape (200 OK)**:
  ```json
  {
    "status": "pass",
    "target_printer_profile": "Prusa_MK4",
    "target_material_profile": "PLA",
    "model": null,
    "issues": [],
    "confidence_level": "medium"
  }
  ```
* **Error Payload (400 / 401 / 500)**:
  ```json
  {
    "error": "Error description message"
  }
  ```

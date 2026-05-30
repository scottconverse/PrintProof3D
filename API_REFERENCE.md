# PrintProof3D Crate API Reference

This document serves as the canonical developer API reference for the PrintProof3D crates. It documents the public data structures, validation traits, adapter protocols, and plugin sandbox APIs.

---

## 1. `printproof3d-core` — Shared Models & Schema

This crate contains all core structures and profile validation rules.

### 1.1 `PrinterProfile` (Struct)
Defines the capabilities, limitations, and configurations of the 3D printer.

| Field | Type | Description |
|:---|:---|:---|
| `manufacturer` | `String` | Manufacturer name (e.g. "Prusa"). |
| `model` | `String` | Specific model identifier (e.g. "MK4"). |
| `protocol_family` | `ProtocolFamily` | Communication family enum (`moonraker`, `octoprint`, `marlin_serial`). |
| `build_volume` | `BuildVolume` | The dimensional bounds of the printbed. |
| `bed_shape` | `BedShape` | Layout boundary layout enum (`rectangular`, `circular`). |
| `nozzle_diameters` | `Vec<f32>` | List of nozzle diameters supported (e.g. `[0.4, 0.6]`). |
| `default_nozzle_diameter` | `f32` | Diameter of the nozzle loaded by default. |
| `min_layer_height` | `f32` | Minimal layer height in mm. |
| `max_layer_height` | `f32` | Maximum layer height in mm. |
| `max_hotend_temp` | `f32` | Maximum safe extruder temperature in Celsius. |
| `max_bed_temp` | `f32` | Maximum safe bed temperature in Celsius. |
| `has_enclosure` | `bool` | Enclosure status. |
| `supports_mmu` | `bool` | Multi-material capabilities indicator. |
| `firmware_flavor` | `FirmwareFlavor` | Firmware type (`marlin`, `klipper`, `reprap`, `bambu`). |
| `supported_file_types` | `Vec<String>` | File extensions accepted (e.g. `["gcode"]`). |
| `unsafe_commands` | `Vec<String>` | List of blacklisted G-code instructions (e.g. `["M500"]`). |

### 1.2 `MaterialProfile` (Struct)
Describes filament properties and temperature ranges.

| Field | Type | Description |
|:---|:---|:---|
| `name` | `String` | Chemical name of filament (e.g. "Polylactic Acid"). |
| `abbreviations` | `Vec<String>` | List of short names (e.g. `["PLA"]`). |
| `min_nozzle_temp` | `f32` | Lower bound safe hotend temp in Celsius. |
| `max_nozzle_temp` | `f32` | Upper bound safe hotend temp in Celsius. |
| `min_bed_temp` | `f32` | Lower bound safe bed temp in Celsius. |
| `max_bed_temp` | `f32` | Upper bound safe bed temp in Celsius. |
| `cooling_fan_speed_pct` | `f32` | Default target fan speed percentage. |
| `warp_risk` | `RiskLevel` | Risk classification (`low`, `medium`, `high`). |
| `min_feature_size_mm` | `f32` | The minimum printable line/feature width in mm. |

### 1.3 `ValidationReport` (Struct)
The data report compiled by validation processes.

| Field | Type | Description |
|:---|:---|:---|
| `status` | `ValidationStatus` | Final assessment status (`pass`, `warning`, `fail`). |
| `target_printer_profile` | `String` | Printer profile model reference used in check. |
| `target_material_profile` | `String` | Material profile reference used in check. |
| `model` | `ModelMetadata` | Geometries metadata (dimensions, filename, shape). |
| `issues` | `Vec<ValidationIssue>` | List of warning/failure items detected. |
| `confidence_level` | `String` | Level of confidence in this result (`high`, `medium`, `low`). |

---

## 2. `printproof3d-printability` — Geometry & Path Analysis

Houses core validation traits and parsers.

### 2.1 `ModelValidator` (Trait)
Exposes the entry point for 3D model geometry analysis (e.g., STL).
```rust
pub trait ModelValidator {
    /// Inspects STL files for manifold watertightness and dimension bounds.
    fn validate_mesh(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}
```

### 2.2 `GcodeValidator` (Trait)
Exposes the entry point for G-code static parsing.
```rust
pub trait GcodeValidator {
    /// Validates G-code instructions against bed bounds and temperature settings.
    fn validate_gcode(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}
```

---

## 3. `printproof3d-adapters` — Communication Protocols

Standardizes telemetry queries and command execution across different printer connection models.

### 3.1 `PrinterAdapter` (Trait)
```rust
#[async_trait]
pub trait PrinterAdapter: Send + Sync {
    /// Connects to the remote printer interface.
    async fn connect(&mut self) -> Result<(), AdapterError>;
    
    /// Closes the connection.
    async fn disconnect(&mut self) -> Result<(), AdapterError>;
    
    /// Queries the latest printer state and temperatures.
    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError>;
    
    /// Uploads a print file to the host/SD card.
    async fn upload_file(&self, local_path: &Path, remote_name: &str) -> Result<String, AdapterError>;
    
    /// Controls printer job status.
    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError>;
    async fn pause_job(&self) -> Result<(), AdapterError>;
    async fn resume_job(&self) -> Result<(), AdapterError>;
    async fn cancel_job(&self) -> Result<(), AdapterError>;
    async fn emergency_stop(&self) -> Result<(), AdapterError>;
}
```

---

## 4. `printproof3d-sdk` — Developer Compliance SDK

Contains conformance validation logic to confirm adapter trait compliance.

### 4.1 Conformance Suit Function
Runs a suite of automated checks against a mock or live adapter instance, verifying states and fault behaviors.
```rust
/// Exercises connection handshake, telemetry polling, and job commands.
pub async fn run_conformance_tests<A: PrinterAdapter>(adapter: &mut A) -> Result<(), String>;
```

---

## 5. `printproof3d-plugins` — WebAssembly Runtime

Restricted, sandboxed runtime for custom validation rule modules.

### 5.1 `PluginEngine` (Struct)
Compiles WASM byte arrays.
* `pub fn new() -> Self` — Creates a new instance.
* `pub fn load_plugin(&self, wasm_bytes: &[u8]) -> Result<LoadedPlugin, String>` — Instantiates a WASM module.

### 5.2 `LoadedPlugin` (Struct)
Executes a loaded plugin inside the sandbox.
* `pub fn execute_validation(&mut self, report_json: &str) -> Result<String, String>` — Pass validation report JSON into guest WASM memory and read back the modified report.

### 5.3 `export_validation_plugin!` (Macro)
Developer macro to export required WASM symbols (`alloc`, `dealloc`, `validate`):
```rust
use printproof3d_plugins::export_validation_plugin;

fn my_rules(report: &mut ValidationReport) {
    // Custom logic
}

export_validation_plugin!(my_rules);
```

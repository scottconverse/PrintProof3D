# PrintProof3D Rust API Reference

This reference documents the public structures, traits, and functions exposed by the PrintProof3D crates.

## 1. `printproof3d-core`

### Structs

#### `PrinterProfile`
Defines printer limits and capacities.
- `manufacturer: String`
- `model: String`
- `protocol_family: ProtocolFamily`
- `build_volume: BuildVolume`
- `bed_shape: BedShape`
- `nozzle_diameters: Vec<f32>`
- `default_nozzle_diameter: f32`
- `min_layer_height: f32`
- `max_layer_height: f32`
- `max_hotend_temp: f32`
- `max_bed_temp: f32`
- `has_enclosure: bool`
- `supports_mmu: bool`
- `firmware_flavor: FirmwareFlavor`
- `supported_file_types: Vec<String>`

#### `MaterialProfile`
Defines thermal thresholds and characteristics of printing filament.
- `name: String`
- `abbreviations: Vec<String>`
- `min_nozzle_temp: f32`
- `max_nozzle_temp: f32`
- `min_bed_temp: f32`
- `max_bed_temp: f32`
- `cooling_fan_speed_pct: f32`
- `warp_risk: Difficulty`
- `enclosure_recommended: bool`
- `dryness_sensitive: bool`

#### `ValidationReport`
The output of validation engines.
- `status: ValidationStatus`
- `target_printer_profile: String`
- `target_material_profile: String`
- `model: ModelMetadata`
- `issues: Vec<ValidationIssue>`
- `confidence_level: String`

---

## 2. `printproof3d-printability`

### Functions

```rust
pub fn check_model() -> &'static str
```
Executes a printability pass on a targeted STL mesh or static G-code file.

---

## 3. `printproof3d-adapters`

### Functions

```rust
pub fn list_adapters() -> Vec<&'static str>
```
Returns a list of supported host and protocol adapters (`"moonraker"`, `"octoprint"`, `"marlin"`).

---

## 4. `printproof3d-sdk`

### Functions

```rust
pub fn sdk_init() -> &'static str
```
Initializes the developer SDK context.

---

## Code Example

```rust
use printproof3d_core::{PrinterProfile, MaterialProfile, BuildVolume, BedShape, ProtocolFamily, FirmwareFlavor};
use printproof3d_sdk::sdk_init;

fn main() {
    // Initialize SDK
    sdk_init();

    // Define configuration
    let printer = PrinterProfile {
        manufacturer: "Custom".to_string(),
        model: "Prusa Clone".to_string(),
        protocol_family: ProtocolFamily::MarlinSerial,
        build_volume: BuildVolume { x: 220.0, y: 220.0, z: 250.0 },
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

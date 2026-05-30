# PrintProof3D: Comprehensive User Manual & Integration Guide

Welcome to the official **PrintProof3D** User Manual and Integration Guide. PrintProof3D is a highly modular, type-safe printability analysis, static verification, and printer management engine written in Rust. 

Unlike traditional slicers and printer controllers that execute files blindly, PrintProof3D serves as a pre-flight safety audit layer. By performing static analysis on 3D models (STL mesh geometry) and toolpaths (G-code files) before they are sent to a physical machine, PrintProof3D prevents physical print failures, material waste, and catastrophic hardware damage (e.g., extruder nozzle crashes, heated bed thermal runaway, mechanical axis binding, and high-wear EEPROM writes).

---

## Table of Contents
1. [Core Architectural Philosophy](#1-core-architectural-philosophy)
2. [Profile Configuration & Schema Specifications](#2-profile-configuration--schema-specifications)
3. [Under the Hood: Verification Logic & Mathematics](#3-under-the-hood-verification-logic--mathematics)
4. [Developer Integration Scenarios](#4-developer-integration-scenarios)
5. [Command Line Interface (CLI) Guide](#5-command-line-interface-cli-guide)
6. [Tutorial: Writing a Custom WASM Validation Plugin](#6-tutorial-writing-a-custom-wasm-validation-plugin)
7. [API & Services Integration (REST & MCP)](#7-api--services-integration-rest--mcp)
8. [Printer Connection Adapters & Compliance Verification](#8-printer-connection-adapters--compliance-verification)

---

## 1. Core Architectural Philosophy

PrintProof3D is engineered on a **defense-in-depth safety model**. Industrial additive manufacturing and hobbyist 3D printing alike face risks ranging from faulty mesh topologies (which crash slicers) to malicious or poorly configured G-code instructions (which can run heaters past thermal bounds, damage mechanical endstops, or burn control boards).

To mitigate these risks, PrintProof3D isolates validation from hardware execution across four distinct layers:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        PrintProof3D Core Engine                        │
├───────────────────┬───────────────────┬────────────────────────────────┤
│  Layer 1: Mesh    │  Layer 2: Path    │  Layer 3: WebAssembly          │
│  Geometry Audit   │  G-Code Parser    │  Sandboxed Custom Rules        │
│  (STL boundaries) │  (Thermal/Motion) │  (Dynamic compilation plugins) │
└─────────┬─────────┴─────────┬─────────┴───────────────┬────────────────┘
          │                   │                         │
          ▼                   ▼                         ▼
┌────────────────────────────────────────────────────────────────────────┐
│                  Unified JSON Validation Report Output                 │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│               Layer 4: Network Printer Connection Adapters             │
│            (State telemetry & automated conformance testing)            │
└────────────────────────────────────────────────────────────────────────┘
```

1. **Static Geometry Audit (STL)**: Evaluates structural integrity, watertightness, orientation, and build plate contact area of raw meshes.
2. **Static Path Audit (G-Code)**: Inspects the actual machine commands line-by-line, compiling coordinate bounding boxes, verifying motion boundaries, and auditing thermal targets.
3. **WebAssembly Guest Sandbox (WASM)**: Allows enterprise developers to inject custom compliance rules (e.g., maximum print volume weights, mandatory safety margins, or forbidden geometries) without modifying the compiler or risking system security.
4. **Hardware Compliance Verification (SDK)**: Validates that third-party printer controllers (e.g., Klipper, Marlin, Duet/RRF) maintain state invariants and execute pause/abort loops reliably under simulated failures.

---

## 2. Profile Configuration & Schema Specifications

To perform validations, PrintProof3D matches your print assets against two configuration structures: the **Printer Profile** (describing physical limits) and the **Material Profile** (describing filament properties).

### 2.1 Printer Profile (`.json`)
The Printer Profile defines the physical boundaries, kinematic properties, thermal capacities, and capabilities of the target machine.

```json
{
  "manufacturer": "Prusa",
  "model": "MK4",
  "protocol_family": "prusa_link",
  "build_volume": {
    "type": "rectangular",
    "x": 250.0,
    "y": 210.0,
    "z": 220.0
  },
  "bed_shape": "rectangular",
  "nozzle_diameters": [0.25, 0.4, 0.6, 0.8],
  "default_nozzle_diameter": 0.4,
  "min_layer_height": 0.05,
  "max_layer_height": 0.3,
  "max_hotend_temp": 300.0,
  "max_bed_temp": 120.0,
  "has_enclosure": false,
  "supports_mmu": true,
  "firmware_flavor": "prusa",
  "supported_file_types": ["gcode", "bgcode"],
  "supports_direct_upload": true,
  "supports_pause_resume": true,
  "supports_cancel": true,
  "supports_job_progress": true,
  "supports_webcam": false,
  "supports_chamber_temp": false,
  "known_quirks": ["long_heatup"],
  "unsafe_commands": ["M500"],
  "filename_restrictions": "^[a-zA-Z0-9_-]+\\.(gcode|bgcode)$"
}
```

#### Field Explanations & Physical Constraints:
* **`build_volume`**: Defines physical space limits. Must match the structure shape:
  * `rectangular`: Requires Cartesian bounds `x` (width), `y` (depth), and `z` (height).
  * `cylindrical`: Requires radial boundary `diameter` and vertical height `z`.
* **`bed_shape`**: Must match the kinematic volume shape. Using a `circular` bed with a `rectangular` volume triggers a profile validation error.
* **`nozzle_diameters`**: Defines available nozzle sizes. The system verifies that slicing profiles match physical nozzle installations.
* **`max_hotend_temp` & `max_bed_temp`**: Core physical thermal safety cutoffs. Temperatures exceeding these limits represent immediate hardware hazards and will block validation. The engine strictly rejects profiles declaring hotend targets $> 500^\circ\text{C}$ or bed targets $> 200^\circ\text{C}$ as physically unsafe.
* **`unsafe_commands`**: Blacklisted G-code instructions. For example, `M500` writes parameters to EEPROM. If run repeatedly in a loop, it will wear out the flash storage of the control board.
* **`filename_restrictions`**: Regular expression to enforce local file storage constraints and block malicious file uploads.

### 2.2 Material Profile (`.json`)
The Material Profile describes the physical, thermal, and chemical behaviors of the printing filament.

```json
{
  "name": "Polylactic Acid",
  "abbreviations": ["PLA", "PLA+"],
  "min_nozzle_temp": 190.0,
  "max_nozzle_temp": 220.0,
  "min_bed_temp": 50.0,
  "max_bed_temp": 60.0,
  "cooling_fan_speed_pct": 100.0,
  "warp_risk": "low",
  "bridge_difficulty": "low",
  "overhang_difficulty": "low",
  "enclosure_recommended": false,
  "dryness_sensitive": false,
  "bed_adhesion_notes": "Requires a clean PEI spring steel sheet",
  "min_feature_size_mm": 0.4
}
```

#### Material Properties & Print Dynamics:
* **`min_nozzle_temp` / `max_nozzle_temp`**: Defines the extrusion window. Heating below the minimum causes cold extrusion, jamming the extruder or grinding the filament. Heating above the maximum causes thermal degradation, crystallization, and carbonization.
* **`min_bed_temp` / `max_bed_temp`**: Defines the crystallization/glass transition window of the polymer. 
* **`warp_risk`**: Defines polymer shrinkage. High-warp materials (e.g., ABS, Nylon) trigger strict warnings if contact area heuristics are low or if an enclosure is missing.
* **`overhang_difficulty` & `bridge_difficulty`**: Influence the geometry validator's slope calculations, adjusting tolerance angles dynamically based on the polymer's cooling properties.

---

## 3. Under the Hood: Verification Logic & Mathematics

PrintProof3D performs rigorous mathematical audits rather than simple structural heuristic scanning. Below is the technical breakdown of the algorithms implemented within `printproof3d-printability`.

### 3.1 STL Mesh Verification Mechanics

STL files define 3D models as a list of triangular facets. The engine parses this facet list and executes five geometric passes.

#### 1. Watertightness & Manifold Mesh Verification
A 3D model is printable only if it represents a closed, physical shell. This requires the mesh to be watertight and manifold.
* **Floating-Point Quantization**: Vertices in STL files are represented as 32-bit floats. Floating-point imprecision can cause identical vertices to differ by tiny fractions (e.g., $2.0000001$ vs $1.9999999$). To solve this, the engine quantizes coordinates to 3 decimal places ($10^{-3}\text{ mm}$) and hashes them into 3D integer coordinate keys:
  $$K_v = \left[ \text{round}(x \times 1000), \text{round}(y \times 1000), \text{round}(z \times 1000) \right]$$
* **Canonical Edge Matching**: For each triangular facet, the engine extracts its three edges. To match edges sharing faces, the vertices of each edge are sorted:
  $$\text{canonical\_edge}(a, b) = \begin{cases} (a, b) & \text{if } a < b \\ (b, a) & \text{otherwise} \end{cases}$$
* **Manifold Criteria**: In a closed, manifold boundary mesh, every edge must be shared by **exactly two** triangles (representing the boundary between two adjacent faces). 
  * If an edge is shared by only **1 face**, there is an open hole in the mesh (non-watertight).
  * If an edge is shared by **3 or more faces**, the mesh contains intersecting planes or self-intersections (non-manifold bifurcation).
  * The validator flags any edge with a count $\neq 2$ as a violation, reporting `MESH_NOT_MANIFOLD` with `Critical` severity.

#### 2. Build Volume Bounds Fitting
The engine audits the absolute spatial reach of the model.
* **Rectangular Volumes**: Verifies that for all vertices $V_i = (x_i, y_i, z_i)$, the coordinates reside inside the positive bounding envelope:
  $$\forall i, \quad 0.0 \le x_i \le X_{limit} \quad \wedge \quad 0.0 \le y_i \le Y_{limit} \quad \wedge \quad 0.0 \le z_i \le Z_{limit}$$
* **Cylindrical Volumes**: delta printers position $(0,0)$ at the center of the bed. The engine converts Cartesian coordinates to a cylindrical radius $R$:
  $$R_i^2 = x_i^2 + y_i^2$$
  $$\forall i, \quad R_i^2 \le \left(\frac{\text{diameter}_{limit}}{2}\right)^2 \quad \wedge \quad 0.0 \le z_i \le Z_{limit}$$
  Exceeding these envelopes triggers a `MODEL_OUT_OF_BOUNDS` error.

#### 3. Steep Overhang Normal Analysis
3D printers construct parts layer-by-layer; extruded material cannot hang in empty space. The engine audits overhang slope angles by analyzing the facet normal vector $\hat{n} = [n_x, n_y, n_z]$ relative to the downward vertical vector $[0, 0, -1]$.

```
      ▲ Z+
      │      Normal vector n 
      │       /
      │      /  Angle θ
      │     / ──┐
      │    /     │
──────┴───*──────┼──────► X/Y
          │\     │
          │ \    │ 
          ▼  \   ▼ Downward Bed Vector [0, 0, -1]
             Overhang slope
```

1. The normal vector must point downwards ($n_z < -0.01$) to be considered an overhang.
2. The facet must be elevated above the printbed ($z_{min} > 0.05\text{ mm}$).
3. The cosine of the tilt angle $\theta$ between the normal and the downward vertical is calculated:
   $$\cos\theta = \frac{-\hat{n}_z}{\|\hat{n}\|}$$
4. The slope limit is determined by the material's cooling performance profile:
   * `Low` cooling difficulty: $45^\circ$ limit ($\cos\theta_{limit} \approx 0.707$)
   * `Medium` cooling difficulty: $50^\circ$ limit ($\cos\theta_{limit} \approx 0.642$)
   * `High` cooling difficulty: $55^\circ$ limit ($\cos\theta_{limit} \approx 0.573$)
5. If $\cos\theta < \cos\theta_{limit}$, the overhang is too steep for raw material bridging, triggering an `OVERHANG_UNSUPPORTED` warning.

#### 4. Flat Ceiling & Bridging Detection
If the tilt angle $\cos\theta \ge 0.99$, the facet is horizontal, facing straight down. If suspended ($z_{min} > 0.05\text{ mm}$), it represents a bridge ceiling. The engine registers this under `BRIDGE_UNSUPPORTED`, recommending specialized bridging speed/cooling configurations.

#### 5. Bed Contact Adhesion Evaluation
To prevent the model from detaching during printing, the bed contact surface area must support the height and volume of the part.
* Contact facets are identified where $z_i < 0.05\text{ mm}$ for all three vertices, and the normal is facing straight down ($\hat{n}_z < -0.9$).
* The area of each contact triangle is computed using the vector cross-product of its edge vectors $\vec{u} = V_1 - V_0$ and $\vec{v} = V_2 - V_0$:
   $$\text{Area} = \frac{1}{2} \|\vec{u} \times \vec{v}\| = \frac{1}{2} \sqrt{(u_y v_z - u_z v_y)^2 + (u_z v_x - u_x v_z)^2 + (u_x v_y - u_y v_x)^2}$$
* The contact footprint area ratio is computed against the 2D bounding footprint area:
   $$\text{Ratio} = \frac{\sum \text{Area}_{contact}}{(x_{max} - x_{min}) \times (y_{max} - y_{min})}$$
* If $\text{Ratio} < 0.05$ ($5\%$) or $\sum \text{Area}_{contact} < 10.0\text{ mm}^2$, the engine flags `POOR_BED_ADHESION`. If the material profile warp risk is `high` (e.g. ABS), this warning is upgraded to `Major` severity.

---

### 3.2 G-Code Analysis Mechanics

The G-code parser reads sliced machine instructions line-by-line, tracking kinematic states and thermal settings.

#### 1. Kinematic Coordinate Tracking
The toolhead coordinate frame ($X, Y, Z$) is compiled statefully:
* **G90 (Absolute Mode)**: Subsequent $X,Y,Z$ values specify coordinates directly.
* **G91 (Relative Mode)**: Subsequent $X,Y,Z$ values add to the accumulated coordinates.
* **G28 (Homing)**: Resets coordinates to $(0.0, 0.0, 0.0)$. If specific axis flags are passed (e.g., `G28 X Y`), only those axes home.
* **G0 / G1 / G2 / G3 (Motion)**: Parses linear and arc segments. After each movement, the validator checks if the target coordinate falls outside the physical build volume, logging a `GCODE_OUT_OF_BOUNDS` error on the exact line number.

#### 2. Thermal Window Compliance
The validator intercepts temperature commands:
* `M104` (Set hotend temp) & `M109` (Set hotend temp and wait).
* `M140` (Set bed temp) & `M190` (Set bed temp and wait).
* **Physical Limit Validation**: If a temperature exceeds `max_hotend_temp` or `max_bed_temp` defined in the printer profile, a `Critical` safety violation is flagged (`HOTEND_TEMP_EXCEEDS_MAX` / `BED_TEMP_EXCEEDS_MAX`).
* **Material Window Validation**: If a positive temperature falls outside the material profile's `min` and `max` limits, a `Major` compatibility alert is logged (`HOTEND_TEMP_OUT_OF_RANGE` / `BED_TEMP_OUT_OF_RANGE`).

---

## 4. Developer Integration Scenarios

PrintProof3D is built for developer integration. Below are four common integration architectures:

### 4.1 Git Pre-Commit Hook (Asset Pipelines)
To ensure that only valid, printable STL models are committed to a shared repository, you can set up a local git pre-commit hook (`.git/hooks/pre-commit`):

```bash
#!/bin/sh
# Pre-commit hook to audit STL assets
STAGED_STLS=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(stl|STL)$')

for stl in $STAGED_STLS; do
  echo "Auditing geometry for $stl..."
  printproof3d validate-model \
    --model "$stl" \
    --printer profiles/prusa_mk4.json \
    --material profiles/pla.json
  
  if [ $? -ne 0 ]; then
    echo "ERROR: STL validation failed for $stl. Commit aborted."
    exit 1
  fi
done
exit 0
```

### 4.2 Slicer Custom Post-Processing Hook
Modern slicers (PrusaSlicer, OrcaSlicer, SuperSlicer) allow running post-processing scripts on the exported G-code before saving or sending it. 

Configure your slicer's **Post-processing scripts** setting to run the validator:
```
printproof3d validate-gcode --printer /path/to/profiles/prusa_mk4.json --gcode
```
If the G-code violates printer limits (e.g. coordinates out of bounds or excessive temperatures), the validator exits with code `1`, causing the slicer export to halt and display the error message.

### 4.3 Centralized Print Farm Validation Hub
In a print farm running multiple machines, clients submit files to a centralized validation server.

```
                  ┌──────────────┐
                  │ Slicer/User  │
                  └──────┬───────┘
                         │ 1. Upload File
                         ▼
             ┌───────────────────────┐
             │ Farm Management App   │
             │ (e.g., OctoFarm,      │
             │  custom dashboard)    │
             └───────────┬───────────┘
                         │ 2. POST /validate/model
                         ▼
             ┌───────────────────────┐
             │  PrintProof3D REST    │
             │  Validation Service   │
             └───────────┬───────────┘
                         │ 3. Returns Report
                         ▼
             ┌───────────────────────┐
             │ Evaluates Report:     │
             │ - PASS: Send to Printer│
             │ - FAIL: Block Queue   │
             └───────────────────────┘
```

The database stores profiles for each active machine. When a print job is requested, the manager queries the PrintProof3D service, passing the file and profiles. Jobs are queued to physical hardware only if the report status returns `Pass`.

### 4.4 MCP-Driven Autonomous Correction
AI coding assistants or automated agents can run `printproof3d mcp`. If validation fails, the agent intercepts the report, identifies the issues (e.g. locating non-manifold coordinates or G-code syntax problems), repairs the G-code or geometry, and re-runs the validation until the report returns a clean `Pass`.

---

## 5. Command Line Interface (CLI) Guide

The `printproof3d` command-line utility provides fast local execution.

### 5.1 Installation & Setup

Before running the commands, compile the workspace binaries or install the package targets globally using Cargo.

#### Global System Installation (Recommended)
Installing globally allows you to run `printproof3d` and the HTTP web server `printproof3d-rest` from any command prompt on your system:
```bash
# Install the CLI tool
cargo install --path crates/cli

# Verify CLI version and path reachability
printproof3d --version

# Install the REST API server daemon
cargo install --path crates/rest

# Verify REST server help interface
printproof3d-rest --help
```
> [!TIP]
> Ensure that Cargo's target binary directory (e.g. `~/.cargo/bin` on Unix-like operating systems or `%USERPROFILE%\.cargo\bin` on Windows) is registered in your environment `PATH` variable.

#### Local Source Building
If you do not wish to copy the binaries to your global path, build the binaries directly in the project build target directory:
```bash
cargo build --release
```
The native compiled binaries will be output at:
* **CLI tool**: `./target/release/printproof3d` (or `./target/release/printproof3d.exe` on Windows)
* **REST server**: `./target/release/printproof3d-rest` (or `./target/release/printproof3d-rest.exe` on Windows)

---

### 5.2 Command Reference

#### 1. Validate 3D STL Mesh Geometry (`validate-model`)
Audits watertightness, coordinates, overhang angles, and bed footprint ratios.
```bash
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json \
  --output reports/tetrahedron_report.json
```
* **Options**:
  * `-m, --model <PATH>`: Absolute/relative path to target STL mesh file.
  * `-p, --printer <PATH>`: Path to printer JSON profile.
  * `-a, --material <PATH>`: Path to material JSON profile.
  * `-o, --output <PATH>`: (Optional) Output path for the JSON validation report. If omitted, prints to stdout.
  * `-l, --plugin <PATH>`: (Optional) Path to a compiled custom rules `.wasm` plugin.

#### 2. Validate Sliced Toolpaths (`validate-gcode`)
Audits G-code coordinates, movement envelopes, homing routines, and thermal limits.
```bash
printproof3d validate-gcode \
  --gcode fixtures/safe_print.gcode \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```
* **Options**:
  * `-g, --gcode <PATH>`: Path to input G-code file.
  * `-p, --printer <PATH>`: Path to printer JSON profile.
  * `-a, --material <PATH>`: (Optional) Path to material JSON profile. If omitted, uses generic safe boundaries.
  * `-o, --output <PATH>`: (Optional) Output path for JSON report.
  * `-l, --plugin <PATH>`: (Optional) Path to custom rules `.wasm` plugin.

### 5.2 Shell Return Codes
The CLI returns standardized exit codes for scripting:
* `0`: Validation passed successfully (Status: `Pass`).
* `1`: Validation failed due to errors or warnings (Status: `Warning` or `Fail`), or due to invalid profile structures or missing files.

---

## 6. Tutorial: Writing a Custom WASM Validation Plugin

Developers can extend PrintProof3D's validation suite by compiling custom validation logic to WebAssembly (WASM). This tutorial guides you through writing a plugin that checks if the model volume is too small, warning the operator to scale up the print.

### 6.1 Guest-Host Serialization Architecture
Custom rules run inside a sandboxed `wasmi` interpreter. The host (PrintProof3D) and guest (WASM Plugin) exchange data over a shared linear memory buffer:

```
HOST (PrintProof3D)                                      GUEST (WASM Plugin)
┌──────────────────┐                                     ┌─────────────────┐
│                  │  1. call alloc(input_len)           │                 │
│                  ├────────────────────────────────────►│                 │
│                  │  2. returns input_ptr offset        │                 │
│                  │◄────────────────────────────────────┤                 │
│ Write JSON string│                                     │                 │
│ to WASM memory   ├────────────────────────────────────►│                 │
│                  │  3. call validate(ptr, len)         │                 │
│                  ├────────────────────────────────────►│ Deserialize JSON│
│                  │                                     │ Apply custom checks
│                  │                                     │ Serialize output│
│                  │  4. returns result_u64              │                 │
│                  │◄────────────────────────────────────┤                 │
│ Read output JSON │                                     │                 │
│ string           │◄────────────────────────────────────┤                 │
│                  │  5. call dealloc(ptr, len)          │                 │
│                  ├────────────────────────────────────►│                 │
└──────────────────┘                                     └─────────────────┘
```

The pointer and length of the output JSON are packed into a single 64-bit unsigned integer (`result_u64`):
$$\text{result\_u64} = (\text{output\_ptr} \ll 32) \mid \text{output\_len}$$

### 6.2 Step-by-Step Implementation

#### Step 1: Initialize the Crate
Create a standard Rust library crate:
```bash
cargo new --lib volume-check-plugin
cd volume-check-plugin
```

#### Step 2: Configure `Cargo.toml`
Set the library's crate type to `cdylib` (C-compatible dynamic library) and add the `printproof3d-core` and `printproof3d-plugins` dependencies.

```toml
[package]
name = "volume-check-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# Point to your local PrintProof3D installation paths
printproof3d-core = { path = "../PrintProof3D/crates/core" }
printproof3d-plugins = { path = "../PrintProof3D/crates/plugins" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

#### Step 3: Implement the Custom Rule
Open `src/lib.rs` and write your validation function. Use the `export_validation_plugin!` macro to generate the `alloc`, `dealloc`, and `validate` wrappers.

```rust
use printproof3d_core::BuildVolume;
use printproof3d_plugins::{
    export_validation_plugin, ValidationReport, ValidationIssue, IssueSeverity, ValidationStatus
};

/// Validation logic targeting small print volumes.
fn audit_minimum_print_volume(report: &mut ValidationReport) {
    // Calculate the bounding box volume in cubic millimeters
    let volume_cubic_mm = match &report.model.bounding_box {
        BuildVolume::Rectangular { x, y, z } => x * y * z,
        BuildVolume::Cylindrical { diameter, z } => {
            let radius = diameter / 2.0;
            std::f32::consts::PI * radius * radius * z
        }
    };

    // Flag a warning if the volume is under 1,000 cubic millimeters (1 cm³)
    if volume_cubic_mm < 1000.0 {
        report.issues.push(ValidationIssue {
            id: "VOLUME_MINIMUM_VIOLATION".to_string(),
            severity: IssueSeverity::Minor,
            message: format!(
                "Print volume ({:.2} mm³) is under the 1000.0 mm³ warning threshold.",
                volume_cubic_mm
            ),
            location: None,
            suggested_fixes: vec![
                "Scale up the model in the slicer by at least 15%.".to_string(),
                "Confirm if printing this sub-millimeter component is intended.".to_string(),
            ],
        });

        // Upgrade status if the print was previously passing
        if report.status == ValidationStatus::Pass {
            report.status = ValidationStatus::Warning;
        }
    }
}

// Generate the WASM entry point wrappers using the macro
export_validation_plugin!(audit_minimum_print_volume);
```

#### Step 4: Compile the WebAssembly Crate
Build the project using the Rust WebAssembly target:
```bash
cargo build --target wasm32-unknown-unknown --release
```
The compiled sandbox binary will be saved at:
`target/wasm32-unknown-unknown/release/volume_check_plugin.wasm`

#### Step 5: Run the Plugin via the CLI
Pass the `.wasm` plugin file to the validator using the `--plugin` flag:
```bash
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json \
  --plugin target/wasm32-unknown-unknown/release/volume_check_plugin.wasm
```
The warning will now appear in your validation report output.

---

## 7. API & Services Integration (REST & MCP)

PrintProof3D runs as a local-loopback daemon or as an MCP server for development integrations.

### 7.1 Axum HTTP REST microservice
The HTTP service coordinates uploads and validates print files on the fly. To start the REST server:
```bash
cargo run --package printproof3d-rest
```
By default, the server binds to `127.0.0.1:3000`.

#### Authentication:
Endpoints are protected by Bearer Token authorization. Secure routes require the header:
```
Authorization: Bearer <token>
```
*Note: The token defaults to `secret_print_token`. Developers can override this by setting the `PRINTPROOF3D_API_TOKEN` environment variable.*

#### Endpoint Reference:
* **`GET /profiles/printers`**: Returns a JSON list of all valid printer profiles.
* **`POST /validate/model`**: Performs a geometry check.
  * **Payload**: `multipart/form-data`
  * **Form Fields**:
    * `model`: (Binary File) The STL mesh file.
    * `printer`: (JSON File) The printer profile.
    * `material`: (JSON File) The material profile.
* **`POST /validate/gcode`**: Performs a G-code path check.
  * **Payload**: `multipart/form-data`
  * **Form Fields**:
    * `gcode`: (Binary File) The G-code file.
    * `printer`: (JSON File) The printer profile.
    * `material`: (Optional JSON File) The material profile.

---

### 7.2 Model Context Protocol (MCP) Server
AI agents (e.g. Cursor, Claude Desktop) can interface with the engine over stdin/stdout using the Model Context Protocol:
```bash
printproof3d mcp
```

#### Supported Tools:
1. **`validate_model_printability`**: Runs mesh audits.
   * **Args**: `model_path`, `printer_profile_path`, `material_profile_path`.
2. **`validate_gcode`**: Runs G-code coordinate and thermal audits.
   * **Args**: `gcode_path`, `printer_profile_path`, `material_profile_path` (optional).
3. **`list_printer_profiles`**: Lists available default hardware profiles.
4. **`explain_validation_report`**: Accepts a validation report JSON string and returns a plain-language summary of safety risks.

#### Configuration Example (Claude Desktop):
To add the tool to Claude Desktop, edit `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "printproof3d": {
      "command": "printproof3d",
      "args": ["mcp"]
    }
  }
}
```

---

## 8. Printer Connection Adapters & Compliance Verification

PrintProof3D provides unified control over printer connection interfaces (e.g. Moonraker/Klipper, OctoPrint, Duet/RepRapFirmware) using the `PrinterAdapter` trait from `printproof3d-adapters`.

### 8.1 The `PrinterAdapter` Trait
Developers can integrate custom communication protocols by implementing the following asynchronous trait:

```rust
#[async_trait]
pub trait PrinterAdapter: Send + Sync {
    /// Establishes the socket/serial connection to the printer controller.
    async fn connect(&mut self) -> Result<(), AdapterError>;
    
    /// Gracefully closes the connection.
    async fn disconnect(&mut self) -> Result<(), AdapterError>;
    
    /// Queries current status, temperatures, progress, and active filename.
    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError>;
    
    /// Uploads a print file to local storage or an SD card.
    async fn upload_file(&self, local_path: &Path, remote_name: &str) -> Result<String, AdapterError>;
    
    /// Initiates print execution.
    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError>;
    
    /// Pauses execution.
    async fn pause_job(&self) -> Result<(), AdapterError>;
    
    /// Resumes execution.
    async fn resume_job(&self) -> Result<(), AdapterError>;
    
    /// Cancels print execution.
    async fn cancel_job(&self) -> Result<(), AdapterError>;
    
    /// Halts power and stops motion immediately.
    async fn emergency_stop(&self) -> Result<(), AdapterError>;
}
```

### 8.2 Automated SDK Compliance Verification
To prevent telemetry drift or command issues, the Developer SDK includes an automated compliance validation harness `run_conformance_tests`. 

The test harness exercises your adapter through a series of state changes, validating behavior under normal operations and simulated connection faults:

```rust
use printproof3d_adapters::PrinterAdapter;
use printproof3d_sdk::run_conformance_tests;

#[tokio::test]
async fn verify_custom_adapter_compliance() {
    // Initialize your custom connection client
    let mut my_adapter = MyOctoPrintClient::new("http://192.168.1.100", "MY_API_KEY");
    
    // Execute compliance checks
    let validation_result = run_conformance_tests(&mut my_adapter).await;
    
    assert!(
        validation_result.is_ok(), 
        "Adapter compliance check failed: {:?}", 
        validation_result.err()
    );
}
```
The suite verifies that:
1. `connect` executes successfully.
2. `get_status` returns non-empty state definitions.
3. Pause, resume, and cancellation sequences complete without errors.
4. `disconnect` cleans up sockets and serial connections.

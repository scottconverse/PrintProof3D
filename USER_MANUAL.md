# PrintProof3D User Manual & Integration Guide

Welcome to the **PrintProof3D User Manual**. This document is split into three parts:
- **Part 1 — For Print Operators (Non-Technical)**: Introduction, configuring profiles, running validation checks, and reading report results.
- **Part 2 — For Technical Operators (Systems Integration)**: Geometry math, G-code travel tracking, WASM memory sandbox protocols, REST API, and MCP configurations.
- **Part 3 — For Developers (Crates & Codebase)**: Local setup, writing custom WASM plugins, implementing connection adapters, and running compliance tests.

---

# Part 1 — For Print Operators

This section is written in plain English for 3D printer operators, workshop managers, and print shop technicians. You do not need any programming experience to follow these steps.

## 1. What is PrintProof3D?
PrintProof3D is a safety utility for 3D printing. In a typical workshop, you download a 3D model, slice it, and send it directly to your printer. If the model has holes, or if the slicer settings are wrong, you risk wasting filament, creating messy "spaghetti" prints, or even damaging your hardware (such as crashing the metal nozzle into the printbed or overheating the components).

PrintProof3D acts as a pre-flight checklist. It inspects your 3D models (STL files) and pre-sliced print files (G-code) before they are sent to the printer to ensure they are safe, watertight, and match the capabilities of your specific machine and filament.

---

## 2. Setting Up Profiles
To validate prints, you must define the target machine and filament using two simple text files: the **Printer Profile** and the **Material Profile**.

### 2.1 The Printer Profile (`.json`)
This file defines the physical dimensions, safety thresholds, and capabilities of your 3D printer. Below is an explanation of what each setting controls:

* **Manufacturer & Model**: Identifies the machine (e.g. "Prusa MK4").
* **Build Volume**: The physical boundaries of the bed.
  * *Rectangular*: Specified as `x` (width), `y` (depth), and `z` (height) in millimeters.
  * *Cylindrical*: Specified as `diameter` and `z` height in millimeters (common for circular beds or delta printers).
* **Bed Shape**: Tells the system the shape of your printbed. This must match the volume (a `circular` bed requires a `cylindrical` volume).
* **Nozzle Diameters**: The nozzle sizes you have available (e.g., `0.4` mm, `0.6` mm). 
* **Max Hotend & Bed Temperatures**: Physical safety cutoffs. If a file attempts to heat the nozzle or bed past these limits, PrintProof3D will flag it as a critical hazard.
* **Unsafe Commands**: A list of codes you wish to block. For example, blocking `M500` prevents print files from saving random settings to your printer's permanent memory, which can wear out the mainboard.

### 2.2 The Material Profile (`.json`)
This file defines the thermal requirements and printing limits of your filament (e.g., PLA, PETG, ABS).
* **Min/Max Nozzle & Bed Temperature**: The safe printing window recommended by the filament manufacturer. Printing outside this window leads to extrusion jams or poor layer adhesion.
* **Warp Risk**: The likelihood of the plastic curling as it cools. High-warp materials (like ABS) trigger strict warnings if the model's footprint touching the bed is too small.
* **Overhang & Bridge Difficulty**: Tells the validator how well this filament can print angles in mid-air.

---

## 3. Running Checks & Reading Reports

### 3.1 Running the CLI Utility
You run checks using the `printproof3d` command in your terminal. 

#### Validating a 3D model (STL):
```bash
printproof3d validate-model \
  --model my_part.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

#### Validating a sliced print file (G-code):
```bash
printproof3d validate-gcode \
  --gcode print_job.gcode \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

### 3.2 Reading the Validation Report
After running a check, PrintProof3D outputs a report. The most important field is the **`status`**, which can return one of three results:

1. **`pass` (Green)**: The file matches all profiles. It is safe to print.
2. **`warning` (Yellow)**: Non-blocking issues were found. You should review them, but the file is not dangerous to the machine. Common warnings include low bed contact area or steep overhangs.
3. **`fail` (Red)**: Critical errors were detected (such as coordinates that exceed your printer's physical dimensions or temperatures that exceed safety limits). Printing is blocked.

Every warning or failure includes a **Suggested Fix** explaining how to resolve it (e.g., "Add supports in your slicer" or "Decrease extrusion temperature").

---

# Part 2 — For Technical Operators

This section covers the technical architecture, mathematical calculations, and remote interfaces for systems administrators and integrations.

## 1. Under the Hood: Mathematical Validation

### 1.1 STL Geometry Auditing
The engine parses STL files and performs geometric verification using the following logic:

* **Vertex Quantization**: To prevent floating-point rounding errors (where adjacent faces don't align due to minor float variances), coordinates are scaled to micrometers and rounded to 3D integer keys:
  $$K_v = \left[ \text{round}(x \times 1000), \text{round}(y \times 1000), \text{round}(z \times 1000) \right]$$
* **Manifold (Watertight) Verifier**: The engine identifies edges by sorting vertex coordinates canonically. In a closed manifold shell, every edge must be shared by **exactly two** triangles. If an edge is shared by only **1 triangle** (a hole) or **3+ triangles** (self-intersection), it flags a `MESH_NOT_MANIFOLD` critical failure.
* **Cylindrical Delta Bed Check**: For circular beds, Cartesian coordinates are converted to a radial distance $R$ from the bed center:
  $$R^2 = x^2 + y^2$$
  If $R^2 > (diameter / 2)^2$ for any vertex, it triggers a `MODEL_OUT_OF_BOUNDS` error.
* **Overhang Slope Tilt Analysis**: The engine checks the angle $\theta$ between the downward normal vector and each triangle face normal $\hat{n} = [n_x, n_y, n_z]$:
  $$\cos\theta = \frac{-\hat{n}_z}{\|\hat{n}\|}$$
  If $\cos\theta < \cos\theta_{limit}$ (where $\theta_{limit}$ is $45^\circ$, $50^\circ$, or $55^\circ$ depending on the material's cooling properties), the overhang is too steep and requires support structures.
* **Bed Adhesion Footprint Ratio**: contact area is calculated using vector cross-products of facets touching the bed surface ($Z < 0.05$ mm):
  $$\text{Area} = \frac{1}{2} \|(V_1 - V_0) \times (V_2 - V_0)\|$$
  If the contact area is $< 5\%$ of the model's 2D bounding footprint area, it flags a `POOR_BED_ADHESION` warning.

### 1.2 G-Code Toolpath Analysis
* **Stateful Coordinates Tracking**: Tracks the toolhead's current $X$, $Y$, and $Z$ positions. It monitors absolute positioning (`G90`), relative positioning (`G91`), homing commands (`G28`), and movement segments (`G0`–`G3`). If any movement places the toolhead outside the build volume, the engine flags a `GCODE_OUT_OF_BOUNDS` error.
* **Thermal Target Monitoring**: Intercepts hotend commands (`M104`/`M109`) and bed commands (`M140`/`M190`). Targets exceeding printer limits flag `Critical` errors, while targets outside material temp windows flag `Major` errors.

---

## 2. Systems Integration & Remote APIs

### 2.1 Axum REST Web Service
Start the REST validation microservice:
```bash
cargo run --package printproof3d-rest
```
* Binds to: `127.0.0.1:3000`
* Security: Enforces Bearer token headers (`Authorization: Bearer secret_print_token`). You can override the token by setting the `PRINTPROOF3D_API_TOKEN` environment variable.

#### Endpoints:
* `GET /profiles/printers` — Lists all valid printer JSON profiles.
* `POST /validate/model` (Multipart) — Performs mesh audit. Accepts `model` (STL file), `printer` (JSON profile), and `material` (JSON profile) fields.
* `POST /validate/gcode` (Multipart) — Performs G-code audit. Accepts `gcode` (file), `printer` (JSON profile), and optional `material` (JSON profile) fields.

### 2.2 Model Context Protocol (MCP) Server
Integrate validation directly into AI agents (like Claude Desktop or Cursor) over standard I/O:
```bash
printproof3d mcp
```
* **Exposed Tools**: `validate_model_printability`, `validate_gcode`, `list_printer_profiles`, and `explain_validation_report`.
* **Claude Desktop Integration**: Add this configuration to your `%APPDATA%\Claude\claude_desktop_config.json`:
  ```json
  {
    "mcpServers": {
      "printproof3d": {
        "command": "C:\\path\\to\\printproof3d.exe",
        "args": ["mcp"]
      }
    }
  }
  ```

---

# Part 3 — For Developers

This section details how to compile, test, write WASM plugins, and contribute to the PrintProof3D codebase.

## 1. Local Setup & Compilation

### Prerequisites
* Rust toolchain (edition 2021).
* WebAssembly target: `rustup target add wasm32-unknown-unknown`.

### Setup Steps
```bash
# Clone the repository
git clone https://github.com/scottconverse/PrintProof3D.git
cd PrintProof3D

# Compile the native targets and validation tests
cargo test --workspace

# Compile the example WASM rules plugin
cargo build --package example-plugin --target wasm32-unknown-unknown --release
```

---

## 2. Writing a Custom WASM Validation Plugin

PrintProof3D supports extending validation using WebAssembly plugins. Plugins are executed inside a sandboxed `wasmi` memory boundary.

### 2.1 The Memory Sharing Protocol
Because WASM runs in an isolated sandbox, data is exchanged by passing pointers to linear memory:
1. The host calls `alloc` on the guest WASM to reserve a memory block.
2. The host writes the serialized `ValidationReport` JSON string into the block.
3. The host executes the guest's `validate(ptr, len)` function.
4. The guest processes the validation, updates the report, allocates an output buffer, writes the result, and returns a packed 64-bit value containing the output pointer and length:
   $$\text{result\_u64} = (\text{output\_ptr} \ll 32) \mid \text{output\_len}$$
5. The host reads the updated JSON string and frees memory using the guest's `dealloc` function.

### 2.2 Step-by-Step Plugin Implementation
1. **Initialize the Crate**:
   Create a standard Rust library:
   ```bash
   cargo new --lib my-plugin
   cd my-plugin
   ```
2. **Configure Cargo.toml**:
   Set `crate-type = ["cdylib"]`:
   ```toml
   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   printproof3d-core = { path = "../PrintProof3D/crates/core" }
   printproof3d-plugins = { path = "../PrintProof3D/crates/plugins" }
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   ```
3. **Write the Rule logic (`src/lib.rs`)**:
   Implement a function targeting `ValidationReport` and export it using the `export_validation_plugin!` macro:
   ```rust
   use printproof3d_plugins::{
       export_validation_plugin, ValidationReport, ValidationIssue, IssueSeverity, ValidationStatus
   };

   fn enforce_safety_margin(report: &mut ValidationReport) {
       // Ensure that model bounding box does not exceed a safety limit
       if report.model.bounding_box.max_x() > 200.0 {
           report.issues.push(ValidationIssue {
               id: "SAFETY_MARGIN_EXCEEDED".to_string(),
               severity: IssueSeverity::Major,
               message: "Model exceeds 200mm safety margin on the X axis.".to_string(),
               location: None,
               suggested_fixes: vec!["Center or scale down the model.".to_string()],
           });
           report.status = ValidationStatus::Fail;
       }
   }

   export_validation_plugin!(enforce_safety_margin);
   ```
4. **Compile to WebAssembly**:
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```
5. **Run the Plugin**:
   ```bash
   printproof3d validate-model \
     --model fixtures/tetrahedron.stl \
     --printer profiles/prusa_mk4.json \
     --material profiles/pla.json \
     --plugin target/wasm32-unknown-unknown/release/my_plugin.wasm
   ```

---

## 3. Custom Printer Connections & Compliance Tests

If you are developing a custom connection adapter (e.g. supporting a new network protocol or firmware interface), you can verify its compliance using the conformance test suite.

1. Implement the `PrinterAdapter` trait on your connection client (from `printproof3d-adapters`).
2. Run the automated compliance suite `run_conformance_tests` in a test block:
   ```rust
   use printproof3d_adapters::PrinterAdapter;
   use printproof3d_sdk::run_conformance_tests;

   #[tokio::test]
   async fn test_my_custom_client() {
       let mut client = MyOctoPrintClient::new("192.168.1.100");
       let result = run_conformance_tests(&mut client).await;
       assert!(result.is_ok(), "Conformance failed: {:?}", result);
   }
   ```
The test suite automatically verifies that connection handshakes, telemetry polling, pauses, resumes, and cancellation actions execute reliably and return standard `AdapterError` responses under simulated failures.

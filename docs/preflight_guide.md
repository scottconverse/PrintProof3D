# PrintProof3D Print Job Preflight Guide

This guide details the `preflight` subcommand, which provides a single, unified print job verification workflow. Users can validate 3D model geometry (STL) or sliced toolpaths (G-code) against printer and material profiles, and execute telemetry/connectivity audits against simulated twin mock servers.

---

## 1. Safety Disclaimer & Limitations

> [!WARNING]
> **Simulator Verification Only**
> PrintProof3D preflight checks verify client conformance and retrieve telemetry via simulator twin mocks (`--simulator <protocol>`).
> - A "pass" status indicates that the model or G-code **passes PrintProof3D profile and file validation checks**.
> - It does **NOT** imply physical print safety, actual print success, or hardware certification.
> - Always perform manual verification and follow printer manufacturer safety instructions before initiating a physical print.

---

## 2. Preflight Subcommand Usage

The `preflight` subcommand compiles validations for STL files and G-code under a unified invocation pattern.

### Command Arguments
* `-m, --model <MODEL>`: Path to a 3D model file (e.g. STL). (Required for model validation; mutually exclusive with `--gcode`).
* `-g, --gcode <GCODE>`: Path to a sliced G-code file. (Required for G-code validation; mutually exclusive with `--model`).
* `-p, --printer <PRINTER>`: Path to the target printer profile JSON file. (Required).
* `-a, --material <MATERIAL>`: Path to the material profile JSON file. (Required for `--model` validations; optional for `--gcode` validations).
* `-o, --output <OUTPUT>`: Optional path to write the prettified JSON validation report.
* `-l, --plugin <PLUGIN>`: Optional path to a custom rules WASM validation plugin.
* `-s, --simulator <PROTOCOL>`: Enable simulator check for a specific protocol. Supported: `rrf`, `octoprint`, `moonraker`, `prusalink`, `bambu`, `serial`.

---

## 3. Coherent Verification Workflow & Examples

Ensure the CLI tool is compiled (e.g., using `cargo build --release`).

### A. STL Model Preflight Validation
Validates model mesh boundaries, watertightness/manifoldness, dimensions, and bed contact area.
```powershell
target/release/printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json
```

### B. Sliced G-code Preflight Validation
Analyzes travel moves, coordinate boundaries against print bed limits, and thermal commands against hotend/bed constraints.
```powershell
target/release/printproof3d.exe preflight --gcode fixtures/safe_print.gcode --printer profiles/prusa_mk4.json
```

### C. Simulator-Twin Connectivity Preflight Validation
Spins up a local twin mock server in-process, tests adapter client connection, extracts printer state telemetry, and shuts down the mock server gracefully.
**You must use a printer profile whose protocol matches the simulator being requested.**

#### 1. PrusaLink (using Prusa MK4 Profile)
```powershell
target/release/printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/prusa_mk4.json --material profiles/pla.json --simulator prusalink
```

#### 2. RepRapFirmware (using Duet RRF Profile)
```powershell
target/release/printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/duet_rrf.json --material profiles/pla.json --simulator rrf
```

#### 3. OctoPrint (using Generic OctoPrint Profile)
```powershell
target/release/printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/generic_octoprint.json --material profiles/pla.json --simulator octoprint
```

#### 4. Moonraker/Klipper (using Voron Klipper Profile)
```powershell
target/release/printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/voron_klipper.json --material profiles/pla.json --simulator moonraker
```

#### 5. Bambu Lab MQTT (using Bambu X1C Profile)
```powershell
target/release/printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/bambu_x1c.json --material profiles/pla.json --simulator bambu
```

#### 6. Marlin Serial (using Ender 3 Serial Profile)
```powershell
target/release/printproof3d.exe preflight --model fixtures/tetrahedron.stl --printer profiles/ender3_serial.json --material profiles/pla.json --simulator serial
```

---

## 4. Exit Codes & JSON Output Contract

### Exit Codes
* `0`: The validation report status is `pass` (all checks pass successfully).
* `1`: The validation report status is `warning` or `fail`, or a validation/connection error occurs.

### JSON Validation Report Schema
The output JSON report contains the following fields:
* `status`: Overall validation outcome (`pass`, `warning`, `fail`).
* `target_printer_profile`: Name of the matched printer profile.
* `target_material_profile`: Name of the matched material profile.
* `model`: Object detailing the validated file's bounding box and units.
* `issues`: Array of detected anomalies. Each issue has:
  * `id`: Code string (e.g. `MESH_NOT_MANIFOLD`, `PRINTER_CONNECTION_FAILED`).
  * `severity`: Severity level (`info`, `minor`, `major`, `critical`, `blocker`).
  * `message`: Human-readable description.
  * `suggested_fixes`: Actionable suggestions.
* `confidence_level`: Level of validation certainty (`low`, `medium`, `high`).
* `sliced_settings_assumed`: Key-value map including `simulator_telemetry` if `--simulator` is queried.

---

## 5. Example JSON Outputs

### Status: `pass` (with Simulator Telemetry)
```json
{
  "status": "pass",
  "target_printer_profile": "Prusa_MK4",
  "target_material_profile": "Polylactic Acid",
  "model": {
    "file_name": "tetrahedron.stl",
    "units": "mm",
    "bounding_box": {
      "min_x": 0.0,
      "min_y": 0.0,
      "min_z": 0.0,
      "max_x": 10.0,
      "max_y": 8.66,
      "max_z": 8.16
    }
  },
  "issues": [],
  "confidence_level": "high",
  "sliced_settings_assumed": {
    "simulator_telemetry": {
      "bed_target": 60.0,
      "bed_temp": 60.0,
      "protocol": "prusalink",
      "state": "Idle",
      "tool_target": 210.0,
      "tool_temp": 210.0
    }
  }
}
```

### Status: `fail` (e.g. Non-manifold model mesh)
```json
{
  "status": "fail",
  "target_printer_profile": "Prusa_MK4",
  "target_material_profile": "Polylactic Acid",
  "model": {
    "file_name": "open_triangle.stl",
    "units": "mm",
    "bounding_box": {
      "min_x": 0.0,
      "min_y": 0.0,
      "min_z": 0.0,
      "max_x": 10.0,
      "max_y": 10.0,
      "max_z": 0.0
    }
  },
  "issues": [
    {
      "id": "MESH_NOT_MANIFOLD",
      "severity": "critical",
      "message": "Model mesh is not watertight/manifold. Found 3 open/non-manifold edges.",
      "location": {
        "region": "mesh_boundaries",
        "geometry": null
      },
      "suggested_fixes": [
        "Repair the 3D model in a mesh editor (e.g. Blender, Netfabb) to make it watertight."
      ]
    }
  ],
  "confidence_level": "medium",
  "sliced_settings_assumed": null
}
```

---

## 6. Profile Management & Compatibility Auditing

For tasks focused purely on profile management or specific isolated compatibility questions, PrintProof3D provides dedicated subcommands separate from `preflight`:

* **Discovery**: Use `list-printers` and `list-materials` to locate profile definitions.
* **Inspection**: Use `inspect-profile <FILE>` to auto-detect and detail profile parameters.
* **Validation**: Use `validate-printer-profile <FILE>` and `validate-material-profile <FILE>` to check profile constraints.
* **Compatibility Check**: Use `check-compatibility --printer <PRINTER> [--material <MAT>] [--model <STL>] [--gcode <GCODE>]` to audit specific alignments without doing a full file preflight verification.

For detailed usage guidelines and examples on these specialized commands, refer to the [PrintProof3D User Manual](../USER_MANUAL.md).

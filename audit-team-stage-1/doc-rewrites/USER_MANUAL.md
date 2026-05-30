# PrintProof3D User Manual

This manual provides detailed instructions on how to configure profiles, validate models, and use the PrintProof3D CLI.

## 1. Profile Configuration

PrintProof3D uses two JSON configurations to evaluate print suitability: **Printer Profiles** and **Material Profiles**.

### Defining a Printer Profile
A printer profile defines build parameters, capabilities, and firmware traits.

Example printer profile (`profiles/prusa_mk4.json`):
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
  "filename_restrictions": null
}
```

### Defining a Material Profile
A material profile defines extrusion properties, thermal windows, and structural difficulties.

Example material profile (`profiles/pla.json`):
```json
{
  "name": "Polylactic Acid",
  "abbreviations": ["PLA"],
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
  "bed_adhesion_notes": "Requires clean PEI sheet",
  "min_feature_size_mm": 0.4
}
```

---

## 2. Model & G-code Validation

PrintProof3D supports validation for STL models (geometric/mesh analysis) and G-code (static path and command safety analysis).

### Validating via CLI

Run the `validate-model` command to inspect a mesh:
```bash
printproof3d validate-model \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

This returns a JSON report indicating validation status:
```json
{
  "status": "pass",
  "target_printer_profile": "Prusa_MK4",
  "target_material_profile": "Polylactic Acid",
  "model": {
    "file_name": "tetrahedron.stl",
    "units": "mm",
    "bounding_box": {
      "type": "rectangular",
      "x": 50.0,
      "y": 50.0,
      "z": 50.0
    }
  },
  "issues": [],
  "confidence_level": "high",
  "sliced_settings_assumed": null
}
```

To validate a G-code file:
```bash
printproof3d validate-gcode \
  --gcode fixtures/cube_safe.gcode \
  --printer profiles/prusa_mk4.json
```

---

## 3. Model Context Protocol (MCP) Server (Planned)

> [!NOTE]
> MCP server integration is a planned Stage 2 feature. In the current Stage 1 release, the server protocol logic and its subcommands (such as `printproof3d mcp`) are mock interfaces and not active.

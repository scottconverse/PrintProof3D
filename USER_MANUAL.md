# PrintProof3D User Manual

This manual provides detailed instructions on how to configure profiles, validate models, and use the PrintProof3D CLI and MCP interfaces.

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
  "build_volume": { "x": 250.0, "y": 210.0, "z": 220.0 },
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
  "warp_risk": "easy",
  "bridge_difficulty": "easy",
  "overhang_difficulty": "easy",
  "enclosure_recommended": false,
  "dryness_sensitive": false,
  "bed_adhesion_notes": "Requires clean PEI sheet",
  "min_feature_size_mm": 0.4
}
```

---

## 2. Model & G-code Validation

PrintProof3D supports validation for STL models (geometric/mesh analysis) and G-code (static path and command safety analysis).

### validating via CLI

Run the `validate` command:
```bash
printproof3d validate \
  --model fixtures/tetrahedron.stl \
  --printer profiles/prusa_mk4.json \
  --material profiles/pla.json
```

This returns a JSON report indicating validation status:
```json
{
  "status": "pass",
  "target_printer_profile": "Prusa MK4",
  "target_material_profile": "Polylactic Acid",
  "model": {
    "file_name": "tetrahedron.stl",
    "units": "mm",
    "bounding_box": { "x": 10.0, "y": 10.0, "z": 10.0 }
  },
  "issues": [],
  "confidence_level": "high"
}
```

---

## 3. Model Context Protocol (MCP) Server

PrintProof3D integrates an MCP server, allowing LLMs to invoke tools for profile validation and print analysis.

### Launching the Server
```bash
printproof3d mcp
```

### Available Tools
- `validate_print`: Takes model paths and profile details to run compatibility checks.
- `list_supported_printers`: Lists built-in printer presets.
- `check_adapters`: Checks connectivity to active OctoPrint or Moonraker targets.
```

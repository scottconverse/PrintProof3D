# PrintProof3D FAQ

## General Questions

### What is PrintProof3D?
PrintProof3D is a software engine designed to run checks on 3D printing models (STL meshes, G-code instructions) against target hardware profiles (Printers) and raw materials (Filaments) to identify failures *before* sending jobs to print.

### Does PrintProof3D replace slicer settings?
PrintProof3D performs validation on raw geometries (STL) and pre-sliced outputs (G-code). It doesn't replace a slicer; it verifies sliced results to make sure they match printer capabilities (e.g. no out-of-bound coordinates or unsafe hotend temperature settings).

---

## Technical Questions

### How are JSON Schemas generated?
The schemas are auto-generated from our Rust structs inside `crates/core` via the `schemars` library. Running `cargo test` regenerates the `.schema.json` files in the `/schemas` directory automatically.

### Which printer firmwares are supported?
PrintProof3D adapters support Marlin serial connectivity, Klipper (via Moonraker API), and OctoPrint. Data schemas support multiple firmware definitions like Bambu, RepRap, CrealityOS, and SDCP.

---

## Troubleshooting

### Why is my G-code model throwing an Out-of-Bounds error?
The G-code static bounds checker scans for `G0`/`G1` positioning commands. If any command references an absolute position coordinate larger than the dimensions defined in your `PrinterProfile.build_volume`, a validation warning or failure issue is generated.

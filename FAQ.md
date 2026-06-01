# PrintProof3D: Technical FAQ

This document addresses common developer and operator questions regarding the architecture, mathematical principles, security model, and integration patterns of PrintProof3D.

---

## 1. General & Conceptual Questions

### What is PrintProof3D?
PrintProof3D is a modular static analysis and integration engine for additive manufacturing. Similar to how a code linter parses source code to identify bugs before compilation, PrintProof3D parses 3D models (STL meshes) and printer instructions (G-code files) to identify profile limit violations, geometry failures, and configuration mismatches.

### Does PrintProof3D replace my slicing software?
No. Slicing software (such as PrusaSlicer, OrcaSlicer, or Cura) is responsible for converting 3D geometry into layers and toolpath movements. PrintProof3D acts as an independent auditor. It verifies that the G-code exported by the slicer does not exceed printer profile parameters (e.g. coordinates outside bed limits, or nozzle heat targets exceeding profile temperature limits) and checks compatibility with your filament properties.

### What are the main benefits of static validation?
1. **Static Validation**: Checks for profile limits, configuration mismatches, and G-code instruction anomalies before sending files to the printer.
2. **Material Conservation**: Catches missing support structures, non-manifold geometry, and poor bed adhesion contact areas before wasting expensive filament.
3. **Firmware Lifespan**: Blacklists G-code commands (like Marlin's `M500`) that cause EEPROM wear when executed repeatedly.
4. **Agent Compatibility**: Provides structural validation reports that AI agents can parse to autonomously correct slicing configurations.

---

## 2. Geometry & Math Auditor Questions

### How does the engine resolve floating-point rounding errors in STL models?
STL files describe 3D models as floats. Float coordinates can suffer from minor rounding variances (e.g., $25.000002$ vs $24.999998$), causing watertight meshes to report boundary leaks.
PrintProof3D resolves this by scaling float vertices by $1000$ and rounding them to the nearest integer, creating a quantized 3D hash key:
$$K_v = \left[ \text{round}(x \times 1000), \text{round}(y \times 1000), \text{round}(z \times 1000) \right]$$
This maps vertices to a strict grid resolution of $1\text{ micron}$ ($0.001\text{ mm}$), ensuring coordinate matching is exact and reliable.

### What is a manifold mesh, and how is it audited?
A mesh is manifold (watertight) if it defines a closed solid shell. The engine validates this by tracking every quantized edge. In a closed manifold model, every edge must be shared by **exactly two** triangles.
* An edge with **1 sharing triangle** represents a hole or an open seam.
* An edge with **3 or more sharing triangles** represents intersecting planes or non-manifold junctions.
Any edge count $\neq 2$ is reported as `MESH_NOT_MANIFOLD` with `Critical` severity.

### How does PrintProof3D evaluate overhangs and bridges?
The engine analyzes the Z component ($N_z$) of each facet's normal vector. If the normal points downward ($N_z < -0.01$) and is elevated above the bed surface ($Z > 0.05\text{ mm}$):
1. **Bridging**: If the normal points straight down ($\cos\theta \ge 0.99$ relative to $[0,0,-1]$), it is classified as a horizontal bridge ceiling.
2. **Overhangs**: If the normal tilt angle $\cos\theta < \cos\theta_{limit}$ (where $\theta_{limit}$ is mapped from the material's cooling properties: $45^\circ$, $50^\circ$, or $55^\circ$), it is flagged as a steep overhang requiring slicer-generated supports.

---

## 3. WebAssembly Sandbox Questions

### How secure is the WASM validation plugin loader?
Extremely secure. The engine executes validation plugins inside the pure-Rust `wasmi` interpreter environment. Guests run in a hermetically sealed sandbox with no access to:
* The host operating system
* Network sockets
* File storage
* Environment variables
All communication is limited to JSON string serialization over a shared linear memory buffer.

### Why use WASM instead of a dynamic scripting language like Python or JavaScript?
* **Performance**: WebAssembly runs compiled code at near-native speeds, allowing custom rules to process large reports containing complex coordinate metadata without performance bottlenecks.
* **Hermetic Sandbox**: WASM provides memory isolation out-of-the-box. Scripting languages typically require complex sandboxing configurations to prevent malicious filesystem or network access.
* **Language Agnosticism**: Developers can write validation rules in any language that compiles to standalone WebAssembly (e.g., AssemblyScript, C/C++, Go, Zig), provided they implement the `alloc`/`dealloc`/`validate` interface.

---

## 4. REST API & Integration Questions

### How do I run the REST server in a production print farm?
For high-volume print farms, run the Axum REST server on a centralized server or as a background service:
1. Set the environment variable `PRINTPROOF3D_API_TOKEN` to a secure API token.
2. Hook your farm management system (e.g. OctoPrint, Mainsail, or a custom dashboard) to perform a multipart POST request to `/validate/model` or `/validate/gcode` before sending files to the printer queue.
3. Deny print execution if the validation report returns a `Fail` status.

### Can PrintProof3D run on embedded devices like a Raspberry Pi?
Yes. The core validation library and CLI are written in Rust, compile into a lightweight native binary, and execute with zero runtime dependencies. Running static G-code validation consumes minimal memory, making it well-suited for single-board computers running Klipper or OctoPrint.

### Does PrintProof3D support generating printer and material profile templates?
Yes. You can generate default template profiles using the CLI subcommands `generate-printer-profile` and `generate-material-profile` with the `--output` option.

### Can I validate an entire directory of profiles at once?
Yes. Use the `validate-profile-directory` command specifying the directory path. It will return a status of each profile file found and exit with `0` if all profiles are valid or `1` if any profile is invalid.

### Are existing endpoints backward compatible in Stage 3?
Yes. Existing REST API endpoints remain fully backward compatible:
- `GET /profiles/printers` remains unauthenticated.
- `POST /validate/model` remains Bearer-authenticated.
- `POST /validate/gcode` remains Bearer-authenticated.

### How do I direct CLI validation output to a file?
All validation, profile, and compatibility subcommands support the `-o` or `--output <FILE>` option to write validation outputs directly to a file instead of stdout/stderr.

# Stage 3 Audit & Verification Plan Report
This report documents the verification plan, expected output contracts, exit codes, and validation rules for Stage 3 of `PrintProof3D`.

- **Target Repository:** `C:\Users\scott\Documents\antigravity\eager-archimedes\PrintProof3D`
- **Scope:** Product Surface Completion + Advanced Validation
- **Status:** Stage 3 plan approved and implemented; final commit pending

---

## 1. Exit Code Specifications
Across all commands in `PrintProof3D`, exit codes are structured as follows:

| Commands | Status | Exit Code | Notes |
| :--- | :--- | :--- | :--- |
| `validate-model` / `validate-gcode` / `preflight` | `pass` | `0` | Success |
| `validate-model` / `validate-gcode` / `preflight` | `warning` / `fail` | `1` | Treated as a validation failure/warning error |
| `check-compatibility` | `pass` / `warning` | `0` | Success (warnings are advisory only) |
| `check-compatibility` | `fail` | `1` | Failure |
| `validate-printer-profile` / `validate-material-profile` | `valid` | `0` | Success |
| `validate-printer-profile` / `validate-material-profile` | `invalid` | `1` | Failure |
| `validate-profile-directory` | All profiles valid | `0` | Success |
| `validate-profile-directory` | Any profile invalid | `1` | Failure |
| Any command | Usage/file loading error | `1` | Failure |

---

## 2. Advanced Validation Rules

### A. Geometry Engine (STL Validation)
1. **`MODEL_OVERSIZED`** (`Critical`): If any dimension of the model's bounding box exceeds the corresponding build volume size of the printer profile.
   - *Rectangular*: `(max_x - min_x) > volume_x || (max_y - min_y) > volume_y || (max_z - min_z) > volume_z`
   - *Cylindrical*: `(max_x - min_x) > diameter || (max_y - min_y) > diameter || (max_z - min_z) > volume_z`
2. **`BELOW_BED_GEOMETRY`** (`Major`): If the model's minimum vertex Z coordinate is below bed level (i.e. $Z < -0.05\text{ mm}$).
3. **`DEGENERATE_TRIANGLES`** (`Minor`): Zero-thickness triangles where the calculated area is $< 1e-6\text{ mm}^2$ or vertices are duplicate.

### B. Toolpath Engine (G-code Validation)
1. **`UNSUPPORTED_FILE_TYPE`** (`Critical`): If the target file's extension (e.g. `.gcode`) is not present in the printer profile's `supported_file_types`.
2. **`UNSAFE_COMMAND_BLOCKED`** (`Critical`): If the file contains any command listed in the printer profile's `unsafe_commands` array (e.g., `M500`).
3. **`MISSING_HOMING`** (`Major`): If a toolpath movement command (`G0`-`G3`) or extrusion command is processed before a homing (`G28`) instruction.
4. **`COLD_EXTRUSION`** (`Major`): If a movement command attempts extrusion (`E > 0.0`) when the nozzle's target temperature is below $170^\circ\text{C}$ or below the material profile's `min_nozzle_temp`.

---

## 3. REST API Router Parity

We will implement Axios-parity endpoints in the Axum REST microservice:
- `GET /profiles/materials`: Lists available material profiles.
- `POST /profiles/inspect`: Inspects an uploaded printer or material JSON profile.
- `POST /profiles/validate/printer`: Validates an uploaded printer profile.
- `POST /profiles/validate/material`: Validates an uploaded material profile.
- `POST /validate/compatibility`: Evaluates compatibility on uploaded assets.

### Backward Compatibility & Regression Tests
- **Existing Endpoints Stability:**
  - Existing endpoints must remain backward compatible:
    - GET /profiles/printers remains unauthenticated.
    - POST /validate/model remains Bearer-authenticated.
    - POST /validate/gcode remains Bearer-authenticated.
  - Add regression tests for those existing endpoints so Stage 3 REST parity does not break them.

---

## 4. Verification Gates
Verification will follow the complete watchdog check sequence:
1. `git status --short --branch`
2. `cargo fmt --all -- --check`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
5. `python devtools/agent_health_check.py`
6. `git diff --check`
7. `git status --short --branch`

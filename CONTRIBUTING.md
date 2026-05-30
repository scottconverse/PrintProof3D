# Contributing to PrintProof3D

We welcome contributions to the PrintProof3D codebase. This document outlines the development guidelines, testing standards, schema generation practices, and commit requirements for contributing to the repository.

---

## 1. Development Setup

### System Prerequisites
To build and test the workspace, install the following dependencies on your development machine:
1. **Rust Toolchain**: Install the stable Rust toolchain (minimum version `1.75` matching Rust edition 2021).
2. **WebAssembly Target**: Install the WASM compilation target for building validation plugins:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
3. **Git**: Used for version control.

### Setup Steps
Clone the repository and verify the initial workspace build:
```bash
# Clone the repository
git clone https://github.com/scottconverse/PrintProof3D.git
cd PrintProof3D

# Verify that the entire workspace builds and tests pass
cargo test --workspace
```

---

## 2. Workspace Crate Architecture

When introducing changes or adding features, ensure they reside in the appropriate crate to maintain decoupling:
* Add core data models or schema configurations to **`crates/core`**.
* Add physical geometry checks, STL parsing logic, or G-code path validators to **`crates/printability`**.
* Add printer communication channels or connection protocols to **`crates/adapters`**.
* Add helper mock environments or conformance test sequences to **`crates/sdk`**.
* Add WASM loading interfaces or sandbox modifications to **`crates/plugins`**.
* Add command parsing options to **`crates/cli`**.
* Add API endpoint route handlers to **`crates/rest`**.

---

## 3. Testing & Automated Schema Compliance

PrintProof3D uses automated testing to enforce domain model and schema invariants.

### 1. Running the Test Suite
Before staging code, run all unit, integration, and conformance tests across the workspace:
```bash
cargo test --workspace
```

### 2. Auto-Generating JSON Schemas
The printer profile, material profile, and validation report JSON schemas are auto-generated from our Rust structs in `crates/core` using `schemars`.
* **Testing Gate**: The workspace tests include an automated generator test (`generate_schemas`). Running `cargo test --workspace` automatically builds and overwrites files in `PrintProof3D/schemas/`.
* **Verification**: Never edit the JSON files in `schemas/` manually. If you modify a field or struct in `crates/core/src/lib.rs`, run `cargo test` to update the schemas. Verify that the updated `.schema.json` matches your intended changes before staging.

---

## 4. Git Hooks Configuration

To prevent non-compilable code, test failures, or outdated schemas from being pushed to the remote repository, we recommend setting up local hooks.

### Pre-Commit Geometry Auditor Hook
Audit staged STL files to prevent invalid models from entering the parts catalog:
Create a file named `.git/hooks/pre-commit` (without extensions) and add:
```bash
#!/bin/sh
# Audit staged mesh geometries
STAGED_STLS=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(stl|STL)$')

for stl in $STAGED_STLS; do
  echo "Auditing $stl..."
  cargo run --package printproof3d-cli -- validate-model \
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

### Pre-Push Workspace Integrity Hook
Verify the full test suite passes before pushing changes:
Create a file named `.git/hooks/pre-push` (without extensions) and add:
```bash
#!/bin/sh
# Compile and test workspace
echo "Running pre-push build and tests..."
cargo test --workspace
if [ $? -ne 0 ]; then
  echo "ERROR: Workspace tests failed. Push aborted."
  exit 1
fi
```

Make both hooks executable:
```bash
chmod +x .git/hooks/pre-commit
chmod +x .git/hooks/pre-push
```

---

## 5. Coding Standards & PR Guidelines

Before submitting a Pull Request, ensure your changes follow these criteria:

### 1. Code Formatting
Format all Rust source files using `rustfmt`:
```bash
cargo fmt --all -- --check
```

### 2. Linting Policies
Check the codebase for common mistakes and stylistic issues using `clippy`. PRs with clippy warnings will be blocked by CI:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### 3. Commit Message Structure
We follow conventional commit format. Commit subject lines should be concise (max 72 chars) and prefixed with a lowercase tag:
* `feat:` for introducing new capabilities.
* `fix:` for fixing validation bugs.
* `docs:` for updating user manuals or spec files.
* `test:` for expanding test suites or conformance mock files.
* `refactor:` for modifying logic without altering public interfaces.

### 4. Public API Changes
If you modify public models in `crates/core` or public traits in `crates/printability` or `crates/adapters`:
* Document all fields, variants, and functions using doc comments (`///`).
* Update the relevant sections in `USER_MANUAL.md`, `ARCHITECTURE.md`, and `API_REFERENCE.md`.

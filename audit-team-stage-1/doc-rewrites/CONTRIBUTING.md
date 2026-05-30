# Contributing to PrintProof3D

We welcome contributions to PrintProof3D. Follow these guidelines to build, test, and submit updates.

## Development Setup

1. **Rust Environment**: Ensure you have Rust and Cargo installed (edition 2021).
2. **Clone the Repo**: Clone the repository and navigate to `PrintProof3D`.

```bash
git clone <repo-url>
cd PrintProof3D
```

---

## Testing and Schema Updates

Any modification to core profile structs in `crates/core/src/lib.rs` requires updating the JSON schemas.

### 1. Run the test suite
```bash
cargo test --workspace
```

### 2. Verify Schema Generation
Running the tests automatically regenerates the files in `PrintProof3D/schemas/`. Do not edit the files in `schemas/` manually. Verify that changes to Rust structs are correctly reflected in these JSON schemas.

---

## Pre-push Hook Setup

To enforce compilation and test coverage checks locally before pushing, we recommend setting up a Git pre-push hook:

Create a file named `.git/hooks/pre-push` (without any extension) and add the following content:

```bash
#!/bin/sh
# Pre-push test runner hook
echo "Running pre-push build and tests..."
cargo test --workspace
if [ $? -ne 0 ]; then
    echo "Error: Tests failed. Push aborted."
    exit 1
fi
```

Make the hook executable:

```bash
chmod +x .git/hooks/pre-push
```

---

## Pull Request Guidelines

1. **Format**: Format all code using `cargo fmt` before staging.
2. **Lint**: Check code against clippy with `cargo clippy`.
3. **Commit Messages**: Write descriptive commit messages matching standard conventions.
4. **Documentation**: If changing public API models, document field changes and update the appropriate guides.

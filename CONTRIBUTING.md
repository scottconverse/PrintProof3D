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

The repository includes a pre-push Git hook to gate invalid updates. Ensure it is executable:

```bash
chmod +x .git/hooks/pre-push
```

This hook executes `cargo test` to verify compilations and unit test runs before pushing to remote repository branches.

---

## Pull Request Guidelines

1. **Format**: Format all code using `cargo fmt` before staging.
2. **Lint**: Check code against clippy with `cargo clippy`.
3. **Commit Messages**: Write descriptive commit messages matching standard conventions.
4. **Documentation**: If changing public API models, document field changes and update the appropriate guides.

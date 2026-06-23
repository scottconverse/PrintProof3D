# PrintProof3D Fuzz Targets

Coverage-guided fuzzing for the two untrusted-input parsers — STL meshes and G-code — using
[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer). These are the file formats the
engine ingests from arbitrary sources (including the REST upload endpoint), so the contract is that
the parsers **never panic** on hostile input — they only ever return `Ok(..)` or `Err(..)`.

This is a standalone crate (its own `[workspace]`) so a normal `cargo build` / `cargo test
--workspace` on a stable toolchain never tries to compile it. Fuzzing requires a **nightly**
toolchain and Linux/macOS (libFuzzer needs the sanitizer runtime; it is not supported on the stable
Windows-MSVC toolchain).

The same code paths are also covered by `proptest` smoke tests in
`crates/printability` (`mod fuzz_smoke`), which **do** run on stable in normal CI as a regression
guard. The targets here are for deeper, coverage-guided exploration.

## Targets

| Target       | Entry point                                              | Surface |
|--------------|----------------------------------------------------------|---------|
| `fuzz_stl`   | `printproof3d_printability::parse_stl_bytes`             | Binary + ASCII STL parsing (offset/length arithmetic, capacity guard) |
| `fuzz_gcode` | `StandardGcodeValidator::validate_gcode` (via temp file) | Stateful G-code motion + thermal parsing |

## Running locally

```bash
# One-time setup
rustup toolchain install nightly
cargo install cargo-fuzz

# Run a target (Ctrl-C to stop). Seed with the existing fixtures for faster coverage:
cargo +nightly fuzz run fuzz_stl   fuzz/corpus/fuzz_stl   ../fixtures
cargo +nightly fuzz run fuzz_gcode fuzz/corpus/fuzz_gcode ../fixtures

# Time-boxed run (matches CI):
cargo +nightly fuzz run fuzz_stl -- -max_total_time=60
```

A crash reproducer is written to `fuzz/artifacts/<target>/`. Re-run a single case with:

```bash
cargo +nightly fuzz run fuzz_stl fuzz/artifacts/fuzz_stl/crash-<hash>
```

## CI

`.github/workflows/fuzz.yml` builds both targets and runs each for a short, time-boxed budget on a
weekly schedule and on manual dispatch (kept off the per-PR path to keep PR CI fast and cheap).

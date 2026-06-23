#![no_main]

use libfuzzer_sys::fuzz_target;

// The STL parser ingests arbitrary, untrusted file bytes (e.g. via the REST upload endpoint). It
// must never panic on malformed input — only ever return Ok(facets) or Err(message). This target
// hammers both the binary-STL offset/length logic and the ASCII fallback.
//
// Seed corpus: fixtures/*.stl. Run with:
//   cargo +nightly fuzz run fuzz_stl
fuzz_target!(|data: &[u8]| {
    let _ = printproof3d_printability::parse_stl_bytes(data);
});

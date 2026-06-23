#![no_main]

use libfuzzer_sys::fuzz_target;
use printproof3d_core::{MaterialProfile, PrinterProfile};
use printproof3d_printability::{GcodeValidator, StandardGcodeValidator};
use std::io::Write;
use std::sync::OnceLock;

// Real printer/material profiles, embedded at compile time so the fuzz loop does no profile I/O.
const PRUSA_MK4_JSON: &str = include_str!("../../profiles/prusa_mk4.json");
const PLA_JSON: &str = include_str!("../../profiles/pla.json");

fn profiles() -> &'static (PrinterProfile, MaterialProfile) {
    static PROFILES: OnceLock<(PrinterProfile, MaterialProfile)> = OnceLock::new();
    PROFILES.get_or_init(|| {
        let printer: PrinterProfile =
            serde_json::from_str(PRUSA_MK4_JSON).expect("embedded printer profile must be valid");
        let material: MaterialProfile =
            serde_json::from_str(PLA_JSON).expect("embedded material profile must be valid");
        (printer, material)
    })
}

// The G-code validator parses arbitrary, untrusted file contents (stateful motion + thermal
// accounting). It must never panic — only return Ok(report) or Err(message). The validator takes a
// path, so each input is staged to a temp `.gcode` file (the `.gcode` extension keeps it past the
// supported-file-type gate so the real parsing path is exercised).
//
// Seed corpus: fixtures/*.gcode. Run with:
//   cargo +nightly fuzz run fuzz_gcode
fuzz_target!(|data: &[u8]| {
    let (printer, material) = profiles();
    let mut tmp = match tempfile::Builder::new().suffix(".gcode").tempfile() {
        Ok(t) => t,
        Err(_) => return,
    };
    if tmp.write_all(data).is_err() {
        return;
    }
    let _ = StandardGcodeValidator.validate_gcode(tmp.path(), printer, material);
});

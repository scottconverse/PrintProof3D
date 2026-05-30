// PrintProof3D Printability Engine
use printproof3d_core::{PrinterProfile, MaterialProfile, ValidationReport};
use std::path::Path;

pub fn check_model() -> &'static str {
    "ok"
}

pub trait ModelValidator {
    fn validate_mesh(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}

pub trait GcodeValidator {
    fn validate_gcode(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}

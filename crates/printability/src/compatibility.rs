use printproof3d_core::{IssueSeverity, MaterialProfile, PrinterProfile, ValidationIssue};

/// Check compatibility between printer profile and material profile properties.
pub fn check_printer_material_compatibility(
    printer: &PrinterProfile,
    material: &MaterialProfile,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // 1. Hotend Temperature check
    if material.min_nozzle_temp > printer.max_hotend_temp
        || material.max_nozzle_temp > printer.max_hotend_temp
    {
        issues.push(ValidationIssue {
            id: "HOTEND_TEMP_INSUFFICIENT".to_string(),
            severity: IssueSeverity::Critical,
            message: format!(
                "Filament recommended nozzle temperature range ({:.1}°C - {:.1}°C) exceeds printer maximum hotend temperature limit ({:.1}°C).",
                material.min_nozzle_temp, material.max_nozzle_temp, printer.max_hotend_temp
            ),
            location: None,
            suggested_fixes: vec![
                "Choose a printer with a higher hotend temperature rating.".to_string(),
                "Use a different filament material compatible with low-temperature printing.".to_string(),
            ],
        });
    }

    // 2. Bed Temperature check
    if material.min_bed_temp > printer.max_bed_temp {
        issues.push(ValidationIssue {
            id: "BED_TEMP_INSUFFICIENT".to_string(),
            severity: IssueSeverity::Critical,
            message: format!(
                "Filament recommended bed temperature range ({:.1}°C - {:.1}°C) exceeds printer maximum bed temperature limit ({:.1}°C).",
                material.min_bed_temp, material.max_bed_temp, printer.max_bed_temp
            ),
            location: None,
            suggested_fixes: vec![
                "Choose a printer with a higher bed temperature limit.".to_string(),
                "Use a different filament material requiring lower bed temperatures.".to_string(),
            ],
        });
    }

    // 3. Enclosure recommendation check
    if material.enclosure_recommended && !printer.has_enclosure {
        issues.push(ValidationIssue {
            id: "ENCLOSURE_REQUIRED".to_string(),
            severity: IssueSeverity::Major,
            message: "Filament recommends an enclosed printing chamber, but target printer is unenclosed.".to_string(),
            location: None,
            suggested_fixes: vec![
                "Install a physical enclosure around the printer before printing.".to_string(),
                "Use draft shields or alternative bed adhesion methods in the slicer.".to_string(),
            ],
        });
    }

    // 4. Nozzle feature size checks
    let has_suitable_nozzle = printer
        .nozzle_diameters
        .iter()
        .any(|&d| d <= material.min_feature_size_mm);

    if !has_suitable_nozzle {
        issues.push(ValidationIssue {
            id: "NOZZLE_DETAIL_UNSUPPORTED".to_string(),
            severity: IssueSeverity::Major,
            message: format!(
                "No supported nozzle diameter is small enough to resolve the filament minimum feature size requirement ({:.2} mm).",
                material.min_feature_size_mm
            ),
            location: None,
            suggested_fixes: vec![
                "Choose a printer that supports smaller nozzle sizes.".to_string(),
                "Use a filament that supports larger detail resolution limits.".to_string(),
            ],
        });
    } else if printer.default_nozzle_diameter > material.min_feature_size_mm {
        issues.push(ValidationIssue {
            id: "NOZZLE_SWAP_REQUIRED".to_string(),
            severity: IssueSeverity::Minor,
            message: format!(
                "Default installed nozzle diameter ({:.2} mm) is larger than recommended filament feature size ({:.2} mm). A nozzle swap is required.",
                printer.default_nozzle_diameter, material.min_feature_size_mm
            ),
            location: None,
            suggested_fixes: vec![
                "Swap the printer's nozzle to a smaller diameter (e.g. 0.25 mm or 0.4 mm) before starting the print.".to_string(),
            ],
        });
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use printproof3d_core::{BedShape, BuildVolume, FirmwareFlavor, ProtocolFamily, RiskLevel};

    fn dummy_printer(
        hotend: f32,
        bed: f32,
        enclosure: bool,
        nozzles: Vec<f32>,
        default_nozzle: f32,
    ) -> PrinterProfile {
        PrinterProfile {
            manufacturer: "Test".to_string(),
            model: "Printer".to_string(),
            protocol_family: ProtocolFamily::PrusaLink,
            build_volume: BuildVolume::Rectangular {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
            bed_shape: BedShape::Rectangular,
            nozzle_diameters: nozzles,
            default_nozzle_diameter: default_nozzle,
            min_layer_height: 0.05,
            max_layer_height: 0.3,
            max_hotend_temp: hotend,
            max_bed_temp: bed,
            has_enclosure: enclosure,
            supports_mmu: false,
            firmware_flavor: FirmwareFlavor::Marlin,
            supported_file_types: vec!["gcode".to_string()],
            supports_direct_upload: false,
            supports_pause_resume: false,
            supports_cancel: false,
            supports_job_progress: false,
            supports_webcam: false,
            supports_chamber_temp: false,
            known_quirks: vec![],
            unsafe_commands: vec![],
            filename_restrictions: None,
        }
    }

    fn dummy_material(
        min_nozzle: f32,
        max_nozzle: f32,
        min_bed: f32,
        enclosure: bool,
        min_feature: f32,
    ) -> MaterialProfile {
        MaterialProfile {
            name: "TestMaterial".to_string(),
            abbreviations: vec!["TM".to_string()],
            min_nozzle_temp: min_nozzle,
            max_nozzle_temp: max_nozzle,
            min_bed_temp: min_bed,
            max_bed_temp: min_bed + 10.0,
            cooling_fan_speed_pct: 100.0,
            warp_risk: RiskLevel::Low,
            bridge_difficulty: RiskLevel::Low,
            overhang_difficulty: RiskLevel::Low,
            enclosure_recommended: enclosure,
            dryness_sensitive: false,
            bed_adhesion_notes: None,
            min_feature_size_mm: min_feature,
        }
    }

    #[test]
    fn test_compatibility_all_pass() {
        let printer = dummy_printer(300.0, 100.0, true, vec![0.4, 0.6], 0.4);
        let material = dummy_material(200.0, 220.0, 60.0, true, 0.4);
        let issues = check_printer_material_compatibility(&printer, &material);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_compatibility_hotend_insufficient() {
        let printer = dummy_printer(240.0, 100.0, true, vec![0.4], 0.4);
        let material = dummy_material(250.0, 260.0, 60.0, false, 0.4);
        let issues = check_printer_material_compatibility(&printer, &material);
        assert!(issues.iter().any(|i| i.id == "HOTEND_TEMP_INSUFFICIENT"));
    }

    #[test]
    fn test_compatibility_bed_insufficient() {
        let printer = dummy_printer(300.0, 50.0, true, vec![0.4], 0.4);
        let material = dummy_material(200.0, 220.0, 60.0, false, 0.4);
        let issues = check_printer_material_compatibility(&printer, &material);
        assert!(issues.iter().any(|i| i.id == "BED_TEMP_INSUFFICIENT"));
    }

    #[test]
    fn test_compatibility_enclosure_required() {
        let printer = dummy_printer(300.0, 100.0, false, vec![0.4], 0.4);
        let material = dummy_material(200.0, 220.0, 60.0, true, 0.4);
        let issues = check_printer_material_compatibility(&printer, &material);
        assert!(issues.iter().any(|i| i.id == "ENCLOSURE_REQUIRED"));
    }

    #[test]
    fn test_compatibility_nozzle_swap_required() {
        let printer = dummy_printer(300.0, 100.0, true, vec![0.25, 0.4, 0.6], 0.6);
        let material = dummy_material(200.0, 220.0, 60.0, false, 0.4);
        let issues = check_printer_material_compatibility(&printer, &material);
        assert!(issues.iter().any(|i| i.id == "NOZZLE_SWAP_REQUIRED"));
    }

    #[test]
    fn test_compatibility_nozzle_detail_unsupported() {
        let printer = dummy_printer(300.0, 100.0, true, vec![0.4, 0.6], 0.4);
        let material = dummy_material(200.0, 220.0, 60.0, false, 0.25);
        let issues = check_printer_material_compatibility(&printer, &material);
        assert!(issues.iter().any(|i| i.id == "NOZZLE_DETAIL_UNSUPPORTED"));
    }
}

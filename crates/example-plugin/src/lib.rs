use printproof3d_core::BuildVolume;
use printproof3d_plugins::{
    export_validation_plugin, ValidationReport, ValidationIssue, IssueSeverity, ValidationStatus
};

fn check_rules(report: &mut ValidationReport) {
    // Flag a warning if the model bounding box volume is extremely small (e.g., < 1000 mm³)
    let bbox = &report.model.bounding_box;
    
    let volume = match bbox {
        BuildVolume::Rectangular { x, y, z } => {
            x * y * z
        }
        BuildVolume::Cylindrical { diameter, z } => {
            let radius = diameter / 2.0;
            std::f32::consts::PI * radius * radius * z
        }
    };
    
    if volume < 1000.0 {
        report.issues.push(ValidationIssue {
            id: "VOLUME_TOO_SMALL".to_string(),
            severity: IssueSeverity::Minor,
            message: format!(
                "Model bounding box volume is extremely small ({:.2} mm³). Verify scale/units.",
                volume
            ),
            location: None,
            suggested_fixes: vec!["Check imported model scale and units (likely needs to be in mm).".to_string()],
        });
        
        if report.status == ValidationStatus::Pass {
            report.status = ValidationStatus::Warning;
        }
    }
}

export_validation_plugin!(check_rules);

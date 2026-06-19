use printproof3d_plugins::{
    export_validation_plugin, IssueSeverity, ValidationIssue, ValidationReport, ValidationStatus,
};

fn check_rules(report: &mut ValidationReport) {
    // Flag a warning if the model bounding box volume is extremely small (e.g., < 1000 mm³)
    let bbox = &report.model.bounding_box;

    let volume = (bbox.max_x - bbox.min_x) * (bbox.max_y - bbox.min_y) * (bbox.max_z - bbox.min_z);

    if volume < 5.0 {
        report.issues.push(ValidationIssue {
            id: "VOLUME_CRITICAL".to_string(),
            severity: IssueSeverity::Critical,
            message: format!(
                "Model bounding box volume is critically small ({:.2} mm³).",
                volume
            ),
            location: None,
            suggested_fixes: vec![],
        });
        // Purposely do not update status here to test host-side invariant re-eval
    } else if volume < 1000.0 {
        report.issues.push(ValidationIssue {
            id: "VOLUME_TOO_SMALL".to_string(),
            severity: IssueSeverity::Minor,
            message: format!(
                "Model bounding box volume is extremely small ({:.2} mm³). Verify scale/units.",
                volume
            ),
            location: None,
            suggested_fixes: vec![
                "Check imported model scale and units (likely needs to be in mm).".to_string(),
            ],
        });

        if report.status == ValidationStatus::Pass {
            report.status = ValidationStatus::Warning;
        }
    }
}

export_validation_plugin!(check_rules);

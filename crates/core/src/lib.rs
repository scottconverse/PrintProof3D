// PrintProof3D Core Crate
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BedShape {
    Rectangular,
    Circular,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolFamily {
    Klipper,
    OctoPrint,
    MarlinSerial,
    PrusaLink,
    RepRapFirmware,
    BambuMqtt,
    ElegooSdcp,
    CrealityOs,
    AnycubicLan,
    FlashForgeTcp,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareFlavor {
    Klipper,
    Marlin,
    RepRapFirmware,
    Prusa,
    Bambu,
    Elegoo,
    Creality,
    Anycubic,
    FlashForge,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct BuildVolume {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PrinterProfile {
    pub manufacturer: String,
    pub model: String,
    pub protocol_family: ProtocolFamily,
    pub build_volume: BuildVolume,
    pub bed_shape: BedShape,
    pub nozzle_diameters: Vec<f32>,
    pub default_nozzle_diameter: f32,
    pub min_layer_height: f32,
    pub max_layer_height: f32,
    pub max_hotend_temp: f32,
    pub max_bed_temp: f32,
    pub has_enclosure: bool,
    pub supports_mmu: bool,
    pub firmware_flavor: FirmwareFlavor,
    pub supported_file_types: Vec<String>,
    
    // Connectivity capabilities
    pub supports_direct_upload: bool,
    pub supports_pause_resume: bool,
    pub supports_cancel: bool,
    pub supports_job_progress: bool,
    pub supports_webcam: bool,
    pub supports_chamber_temp: bool,
    
    // Quirks and constraints
    pub known_quirks: Vec<String>,
    pub unsafe_commands: Vec<String>,
    pub filename_restrictions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MaterialProfile {
    pub name: String,
    pub abbreviations: Vec<String>,
    pub min_nozzle_temp: f32,
    pub max_nozzle_temp: f32,
    pub min_bed_temp: f32,
    pub max_bed_temp: f32,
    pub cooling_fan_speed_pct: f32,
    pub warp_risk: Difficulty,
    pub bridge_difficulty: Difficulty,
    pub overhang_difficulty: Difficulty,
    pub enclosure_recommended: bool,
    pub dryness_sensitive: bool,
    pub bed_adhesion_notes: Option<String>,
    pub min_feature_size_mm: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Blocker,
    Critical,
    Major,
    Minor,
    Nit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ModelMetadata {
    pub file_name: String,
    pub units: String,
    pub bounding_box: BuildVolume,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct IssueLocation {
    pub region: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ValidationIssue {
    pub id: String,
    pub severity: IssueSeverity,
    pub message: String,
    pub location: Option<IssueLocation>,
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ValidationReport {
    pub status: ValidationStatus,
    pub target_printer_profile: String,
    pub target_material_profile: String,
    pub model: ModelMetadata,
    pub issues: Vec<ValidationIssue>,
    pub confidence_level: String,
    pub sliced_settings_assumed: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }

    #[test]
    fn test_printer_profile_serialization() {
        let profile = PrinterProfile {
            manufacturer: "Prusa".to_string(),
            model: "MK4".to_string(),
            protocol_family: ProtocolFamily::PrusaLink,
            build_volume: BuildVolume { x: 250.0, y: 210.0, z: 220.0 },
            bed_shape: BedShape::Rectangular,
            nozzle_diameters: vec![0.25, 0.4, 0.6, 0.8],
            default_nozzle_diameter: 0.4,
            min_layer_height: 0.05,
            max_layer_height: 0.30,
            max_hotend_temp: 300.0,
            max_bed_temp: 120.0,
            has_enclosure: false,
            supports_mmu: true,
            firmware_flavor: FirmwareFlavor::Prusa,
            supported_file_types: vec!["gcode".to_string(), "bgcode".to_string()],
            supports_direct_upload: true,
            supports_pause_resume: true,
            supports_cancel: true,
            supports_job_progress: true,
            supports_webcam: false,
            supports_chamber_temp: false,
            known_quirks: vec!["long_heatup".to_string()],
            unsafe_commands: vec!["M500".to_string()],
            filename_restrictions: None,
        };

        let json = serde_json::to_string_pretty(&profile).unwrap();
        let deserialized: PrinterProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, deserialized);
    }

    #[test]
    fn test_material_profile_serialization() {
        let material = MaterialProfile {
            name: "Polylactic Acid".to_string(),
            abbreviations: vec!["PLA".to_string()],
            min_nozzle_temp: 190.0,
            max_nozzle_temp: 220.0,
            min_bed_temp: 50.0,
            max_bed_temp: 60.0,
            cooling_fan_speed_pct: 100.0,
            warp_risk: Difficulty::Easy,
            bridge_difficulty: Difficulty::Easy,
            overhang_difficulty: Difficulty::Easy,
            enclosure_recommended: false,
            dryness_sensitive: false,
            bed_adhesion_notes: Some("Requires clean PEI sheet".to_string()),
            min_feature_size_mm: 0.4,
        };

        let json = serde_json::to_string_pretty(&material).unwrap();
        let deserialized: MaterialProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(material, deserialized);
    }

    #[test]
    fn test_validation_report_serialization() {
        let report = ValidationReport {
            status: ValidationStatus::Warning,
            target_printer_profile: "prusa_mk4_default".to_string(),
            target_material_profile: "generic_pla".to_string(),
            model: ModelMetadata {
                file_name: "test_bracket.stl".to_string(),
                units: "mm".to_string(),
                bounding_box: BuildVolume { x: 50.0, y: 30.0, z: 20.0 },
            },
            issues: vec![
                ValidationIssue {
                    id: "OVERHANG_UNSUPPORTED".to_string(),
                    severity: IssueSeverity::Major,
                    message: "Unsupported overhang exceeds 45 degrees.".to_string(),

                    location: Some(IssueLocation {
                        region: "underside_flange".to_string(),
                        x: Some(12.5),
                        y: Some(5.0),
                        z: Some(0.0),
                    }),
                    suggested_fixes: vec!["enable_supports".to_string(), "rotate_model_90_y".to_string()],
                }
            ],
            confidence_level: "high".to_string(),
            sliced_settings_assumed: None,
        };

        let json = serde_json::to_string_pretty(&report).unwrap();
        let deserialized: ValidationReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, deserialized);
    }

    #[test]
    fn generate_schemas() {
        use schemars::schema_for;
        use std::fs::create_dir_all;
        use std::fs::File;
        use std::io::Write;
        use std::path::Path;

        let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
        create_dir_all(&schema_dir).unwrap();

        let printer_schema = schema_for!(PrinterProfile);
        let mut file = File::create(schema_dir.join("printer_profile.schema.json")).unwrap();
        file.write_all(serde_json::to_string_pretty(&printer_schema).unwrap().as_bytes()).unwrap();

        let material_schema = schema_for!(MaterialProfile);
        let mut file = File::create(schema_dir.join("material_profile.schema.json")).unwrap();
        file.write_all(serde_json::to_string_pretty(&material_schema).unwrap().as_bytes()).unwrap();

        let report_schema = schema_for!(ValidationReport);
        let mut file = File::create(schema_dir.join("validation_report.schema.json")).unwrap();
        file.write_all(serde_json::to_string_pretty(&report_schema).unwrap().as_bytes()).unwrap();
    }
}

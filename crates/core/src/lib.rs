// PrintProof3D Core Crate
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod connection;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The visual geometric shape of the printer bed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BedShape {
    Rectangular,
    Circular,
    Custom(String),
}

/// The connection protocol family used to communicate with the printer or print host.
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

/// The firmware flavor running on the machine control board.
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

/// 3D dimensional bounds representation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuildVolume {
    Rectangular {
        /// Bounding box X-axis dimension (width) in millimeters.
        x: f32,
        /// Bounding box Y-axis dimension (depth) in millimeters.
        y: f32,
        /// Bounding box Z-axis dimension (height) in millimeters.
        z: f32,
    },
    Cylindrical {
        /// Diameter of the build volume in millimeters.
        diameter: f32,
        /// Height of the build volume in millimeters.
        z: f32,
    },
}

/// Defines printer properties, capability bounds, and communication protocols.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PrinterProfile {
    /// The manufacturer name of the 3D printer (e.g. "Prusa").
    pub manufacturer: String,
    /// The specific model name of the 3D printer (e.g. "MK4").
    pub model: String,
    /// The network/serial communication protocol standard.
    pub protocol_family: ProtocolFamily,
    /// The printable bounding box volume dimensions in mm.
    pub build_volume: BuildVolume,
    /// The physical shape layout of the build plate.
    pub bed_shape: BedShape,
    /// List of supported nozzle diameter configurations (e.g. [0.4, 0.6]).
    pub nozzle_diameters: Vec<f32>,
    /// Default installed nozzle diameter on the toolhead in mm.
    pub default_nozzle_diameter: f32,
    /// Minimum practical layer height support in mm.
    pub min_layer_height: f32,
    /// Maximum practical layer height support in mm.
    pub max_layer_height: f32,
    /// Maximum safe hotend temperature limit in Celsius.
    pub max_hotend_temp: f32,
    /// Maximum safe heated bed temperature limit in Celsius.
    pub max_bed_temp: f32,
    /// True if the print chamber is fully enclosed.
    pub has_enclosure: bool,
    /// True if an automatic material system or multi-material unit is connected.
    pub supports_mmu: bool,
    /// The internal firmware parser flavor.
    pub firmware_flavor: FirmwareFlavor,
    /// List of file extensions supported for direct execution (e.g. ["gcode"]).
    pub supported_file_types: Vec<String>,

    // Connectivity capabilities
    /// Direct remote print upload connectivity.
    pub supports_direct_upload: bool,
    /// Job pause and resume state control support.
    pub supports_pause_resume: bool,
    /// Active job cancellation support.
    pub supports_cancel: bool,
    /// Live print percentage and telemetry reporting support.
    pub supports_job_progress: bool,
    /// Webcam remote monitoring streaming availability.
    pub supports_webcam: bool,
    /// Active chamber temperature monitoring availability.
    pub supports_chamber_temp: bool,

    // Quirks and constraints
    /// Known configuration or driver bugs to bypass.
    pub known_quirks: Vec<String>,
    /// Slicer blacklisted G-code instructions.
    pub unsafe_commands: Vec<String>,
    /// Target file name constraints regular expression pattern.
    pub filename_restrictions: Option<String>,
}

impl PrinterProfile {
    /// Validates field ranges and constraints for logical correctness.
    pub fn validate(&self) -> Result<(), String> {
        if self.manufacturer.is_empty() {
            return Err("Manufacturer name cannot be empty".to_string());
        }
        if self.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        match &self.build_volume {
            BuildVolume::Rectangular { x, y, z } => {
                if *x <= 0.0 || *y <= 0.0 || *z <= 0.0 {
                    return Err("Build volume dimensions must be positive".to_string());
                }
            }
            BuildVolume::Cylindrical { diameter, z } => {
                if *diameter <= 0.0 || *z <= 0.0 {
                    return Err("Build volume dimensions must be positive".to_string());
                }
            }
        }
        match (&self.bed_shape, &self.build_volume) {
            (BedShape::Circular, BuildVolume::Rectangular { .. }) => {
                return Err("Circular bed shape requires Cylindrical build volume".to_string());
            }
            (BedShape::Rectangular, BuildVolume::Cylindrical { .. }) => {
                return Err("Rectangular bed shape requires Rectangular build volume".to_string());
            }
            _ => {}
        }
        if self.default_nozzle_diameter <= 0.0 {
            return Err("Default nozzle diameter must be positive".to_string());
        }
        for dia in &self.nozzle_diameters {
            if *dia <= 0.0 {
                return Err("All nozzle diameters must be positive".to_string());
            }
        }
        if !self
            .nozzle_diameters
            .contains(&self.default_nozzle_diameter)
        {
            return Err(
                "Default nozzle diameter must be present in nozzle_diameters options".to_string(),
            );
        }
        if self.min_layer_height <= 0.0 || self.max_layer_height <= 0.0 {
            return Err("Layer heights must be positive".to_string());
        }
        if self.min_layer_height > self.max_layer_height {
            return Err(
                "Minimum layer height cannot be greater than maximum layer height".to_string(),
            );
        }
        if self.max_hotend_temp <= 0.0 || self.max_bed_temp <= 0.0 {
            return Err("Maximum temperatures must be positive".to_string());
        }
        if self.max_hotend_temp > 500.0 {
            return Err("Unsafe hotend maximum temperature target (exceeds 500C)".to_string());
        }
        if self.max_bed_temp > 200.0 {
            return Err("Unsafe bed maximum temperature target (exceeds 200C)".to_string());
        }
        Ok(())
    }
}

/// Severity risk scales.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Core material printing window, ventilation, and adhesion configurations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MaterialProfile {
    /// Readable name of the material (e.g. "Polylactic Acid").
    pub name: String,
    /// Known abbreviations (e.g. ["PLA", "PLA+"]).
    pub abbreviations: Vec<String>,
    /// Minimum recommended nozzle temperature in Celsius.
    pub min_nozzle_temp: f32,
    /// Maximum recommended nozzle temperature in Celsius.
    pub max_nozzle_temp: f32,
    /// Minimum recommended bed temperature in Celsius.
    pub min_bed_temp: f32,
    /// Maximum recommended bed temperature in Celsius.
    pub max_bed_temp: f32,
    /// Extruder cooling fan speed percentage (0.0 to 100.0).
    pub cooling_fan_speed_pct: f32,
    /// Relative warp risk under standard airflow conditions.
    pub warp_risk: RiskLevel,
    /// Bridge print extrusion difficulty.
    pub bridge_difficulty: RiskLevel,
    /// Overhang print angle cooling difficulty.
    pub overhang_difficulty: RiskLevel,
    /// True if an enclosure is recommended for this material.
    pub enclosure_recommended: bool,
    /// True if the raw material absorbs ambient moisture easily (hygroscopic).
    pub dryness_sensitive: bool,
    /// Help descriptions for bed preparation.
    pub bed_adhesion_notes: Option<String>,
    /// Smallest resolvable detailed dimension in mm.
    pub min_feature_size_mm: f32,
}

impl MaterialProfile {
    /// Validates thermal bounds and fan limits.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Material name cannot be empty".to_string());
        }
        if self.min_nozzle_temp <= 0.0 || self.max_nozzle_temp <= 0.0 {
            return Err("Nozzle temperatures must be positive".to_string());
        }
        if self.min_nozzle_temp > self.max_nozzle_temp {
            return Err(
                "Minimum nozzle temperature cannot be greater than maximum nozzle temperature"
                    .to_string(),
            );
        }
        if self.min_bed_temp < 0.0 || self.max_bed_temp < 0.0 {
            return Err("Bed temperatures must be non-negative".to_string());
        }
        if self.min_bed_temp > self.max_bed_temp {
            return Err(
                "Minimum bed temperature cannot be greater than maximum bed temperature"
                    .to_string(),
            );
        }
        if self.cooling_fan_speed_pct < 0.0 || self.cooling_fan_speed_pct > 100.0 {
            return Err("Cooling fan speed must be between 0 and 100 percent".to_string());
        }
        if self.min_feature_size_mm <= 0.0 {
            return Err("Minimum feature size must be positive".to_string());
        }
        Ok(())
    }
}

/// Overall print verification result states.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Pass,
    Warning,
    Fail,
}

/// Classification of issue urgency.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Blocker,
    Critical,
    Major,
    Minor,
    Nit,
}

/// Verified file metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ModelMetadata {
    /// Filename of the imported part.
    pub file_name: String,
    /// Length units used (typically "mm").
    pub units: String,
    /// Bounding box layout bounds.
    pub bounding_box: BoundingBox,
}

/// A 3D bounding box region representation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct BoundingBox {
    pub min_x: f32,
    pub min_y: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
}

/// A 3D triangle representation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Triangle {
    pub v0: [f32; 3],
    pub v1: [f32; 3],
    pub v2: [f32; 3],
}

/// The geometric shape/details of the issue location.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocationGeometry {
    Point { x: f32, y: f32, z: f32 },
    BoundingBox(BoundingBox),
    Triangles { triangles: Vec<Triangle> },
}

/// Spatial location coordinates or geometric boundaries of a printability alert.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct IssueLocation {
    /// Geometry region descriptor (e.g. "base", "overhang").
    pub region: String,
    /// Detailed geometric shape representing the issue.
    pub geometry: Option<LocationGeometry>,
}

/// Single compatibility issue item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ValidationIssue {
    /// System ID identifying the error class (e.g. "OVERHANG_UNSUPPORTED").
    pub id: String,
    /// Issue severity layer.
    pub severity: IssueSeverity,
    /// Human readable issue explanation.
    pub message: String,
    /// Millimeter coordinates of the issue.
    pub location: Option<IssueLocation>,
    /// Suggestion fixes.
    pub suggested_fixes: Vec<String>,
}

/// Consolidated printability report.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ValidationReport {
    /// General status of validation.
    pub status: ValidationStatus,
    /// Associated printer profile.
    pub target_printer_profile: String,
    /// Associated material profile.
    pub target_material_profile: String,
    /// Imported model metadata.
    pub model: ModelMetadata,
    /// List of validation failures/warnings.
    pub issues: Vec<ValidationIssue>,
    /// Validation confidence level.
    pub confidence_level: String,
    /// Associated slicer assumptions used.
    pub sliced_settings_assumed: Option<serde_json::Value>,
}

impl ValidationReport {
    /// Enforces state invariants (e.g., Blockers/Criticals force Fail status).
    pub fn validate(&self) -> Result<(), String> {
        let has_critical_or_blocker = self.issues.iter().any(|issue| {
            issue.severity == IssueSeverity::Blocker || issue.severity == IssueSeverity::Critical
        });

        if has_critical_or_blocker && self.status != ValidationStatus::Fail {
            return Err(
                "Report status must be 'fail' if blocker or critical issues exist".to_string(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.5.0-rc3");
    }

    #[test]
    fn test_printer_profile_validation() {
        let profile = PrinterProfile {
            manufacturer: "Prusa".to_string(),
            model: "MK4".to_string(),
            protocol_family: ProtocolFamily::PrusaLink,
            build_volume: BuildVolume::Rectangular {
                x: 250.0,
                y: 210.0,
                z: 220.0,
            },
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

        assert!(profile.validate().is_ok());

        let mut bad_profile = profile.clone();
        bad_profile.build_volume = BuildVolume::Rectangular {
            x: -10.0,
            y: 210.0,
            z: 220.0,
        };
        assert!(bad_profile.validate().is_err());

        let mut bad_temp = profile.clone();
        bad_temp.max_hotend_temp = 600.0;
        assert!(bad_temp.validate().is_err());

        let mut bad_bed_vol = profile.clone();
        bad_bed_vol.bed_shape = BedShape::Circular; // Circular bed + Rectangular volume -> invalid
        assert!(bad_bed_vol.validate().is_err());

        let mut bad_bed_vol2 = profile.clone();
        bad_bed_vol2.build_volume = BuildVolume::Cylindrical {
            diameter: 200.0,
            z: 200.0,
        }; // Rectangular bed + Cylindrical volume -> invalid
        assert!(bad_bed_vol2.validate().is_err());

        let mut bad_nozzle = profile.clone();
        bad_nozzle.nozzle_diameters = vec![0.4, -0.2]; // Negative nozzle diameter -> invalid
        assert!(bad_nozzle.validate().is_err());
    }

    #[test]
    fn test_material_profile_validation() {
        let material = MaterialProfile {
            name: "Polylactic Acid".to_string(),
            abbreviations: vec!["PLA".to_string()],
            min_nozzle_temp: 190.0,
            max_nozzle_temp: 220.0,
            min_bed_temp: 50.0,
            max_bed_temp: 60.0,
            cooling_fan_speed_pct: 100.0,
            warp_risk: RiskLevel::Low,
            bridge_difficulty: RiskLevel::Low,
            overhang_difficulty: RiskLevel::Low,
            enclosure_recommended: false,
            dryness_sensitive: false,
            bed_adhesion_notes: Some("Requires clean PEI sheet".to_string()),
            min_feature_size_mm: 0.4,
        };

        assert!(material.validate().is_ok());

        let mut bad_fan = material.clone();
        bad_fan.cooling_fan_speed_pct = 150.0;
        assert!(bad_fan.validate().is_err());
    }

    #[test]
    fn test_validation_report_invariants() {
        let report = ValidationReport {
            status: ValidationStatus::Pass, // INVARIANT INCONSISTENCY
            target_printer_profile: "prusa_mk4_default".to_string(),
            target_material_profile: "generic_pla".to_string(),
            model: ModelMetadata {
                file_name: "test_bracket.stl".to_string(),
                units: "mm".to_string(),
                bounding_box: BoundingBox {
                    min_x: 0.0,
                    min_y: 0.0,
                    min_z: 0.0,
                    max_x: 50.0,
                    max_y: 30.0,
                    max_z: 20.0,
                },
            },
            issues: vec![ValidationIssue {
                id: "OVERHANG_UNSUPPORTED".to_string(),
                severity: IssueSeverity::Critical,
                message: "Critical unsupported overhang.".to_string(),
                location: None,
                suggested_fixes: vec![],
            }],
            confidence_level: "high".to_string(),
            sliced_settings_assumed: None,
        };

        // Report should fail invariant checks due to critical issue on passing status
        assert!(report.validate().is_err());
    }

    #[test]
    fn test_validation_report_serialization_roundtrip() {
        let report = ValidationReport {
            status: ValidationStatus::Warning,
            target_printer_profile: "prusa_mk4_default".to_string(),
            target_material_profile: "generic_pla".to_string(),
            model: ModelMetadata {
                file_name: "test_bracket.stl".to_string(),
                units: "mm".to_string(),
                bounding_box: BoundingBox {
                    min_x: 0.0,
                    min_y: 0.0,
                    min_z: 0.0,
                    max_x: 50.0,
                    max_y: 30.0,
                    max_z: 20.0,
                },
            },
            issues: vec![ValidationIssue {
                id: "OVERHANG_UNSUPPORTED".to_string(),
                severity: IssueSeverity::Major,
                message: "Steep overhang.".to_string(),
                location: Some(IssueLocation {
                    region: "overhangs".to_string(),
                    geometry: Some(LocationGeometry::Triangles {
                        triangles: vec![Triangle {
                            v0: [0.0, 0.0, 0.0],
                            v1: [1.0, 0.0, 0.0],
                            v2: [0.0, 1.0, 0.0],
                        }],
                    }),
                }),
                suggested_fixes: vec![],
            }],
            confidence_level: "high".to_string(),
            sliced_settings_assumed: None,
        };

        // Serialize report to string
        let serialized = serde_json::to_string(&report);
        assert!(
            serialized.is_ok(),
            "Serialization failed: {:?}",
            serialized.err()
        );
        let serialized_str = serialized.unwrap();

        // Deserialize report from string
        let deserialized: Result<ValidationReport, _> = serde_json::from_str(&serialized_str);
        assert!(
            deserialized.is_ok(),
            "Deserialization failed: {:?}",
            deserialized.err()
        );
        let deserialized_report = deserialized.unwrap();

        // Verify matches original
        assert_eq!(deserialized_report, report);
    }

    #[test]
    fn generate_schemas() {
        use schemars::schema_for;
        use std::fs::create_dir_all;
        use std::path::Path;

        let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");

        let printer_schema = schema_for!(PrinterProfile);
        let printer_schema_str = serde_json::to_string_pretty(&printer_schema).unwrap();
        let printer_path = schema_dir.join("printer_profile.schema.json");

        let material_schema = schema_for!(MaterialProfile);
        let material_schema_str = serde_json::to_string_pretty(&material_schema).unwrap();
        let material_path = schema_dir.join("material_profile.schema.json");

        let report_schema = schema_for!(ValidationReport);
        let report_schema_str = serde_json::to_string_pretty(&report_schema).unwrap();
        let report_path = schema_dir.join("validation_report.schema.json");

        let connection_schema = schema_for!(connection::PrinterConnectionConfig);
        let connection_schema_str = serde_json::to_string_pretty(&connection_schema).unwrap();
        let connection_path = schema_dir.join("connection_config.schema.json");

        if std::env::var("UPDATE_SCHEMAS").is_ok() {
            create_dir_all(&schema_dir).unwrap();
            std::fs::write(&printer_path, &printer_schema_str).unwrap();
            std::fs::write(&material_path, &material_schema_str).unwrap();
            std::fs::write(&report_path, &report_schema_str).unwrap();
            std::fs::write(&connection_path, &connection_schema_str).unwrap();
        } else {
            let read_schema = |path: &Path| -> String {
                std::fs::read_to_string(path)
                    .unwrap_or_default()
                    .replace("\r\n", "\n")
            };

            assert_eq!(
                read_schema(&printer_path),
                printer_schema_str.replace("\r\n", "\n"),
                "Schema mismatch for printer_profile. Run with UPDATE_SCHEMAS=1 to update."
            );
            assert_eq!(
                read_schema(&material_path),
                material_schema_str.replace("\r\n", "\n"),
                "Schema mismatch for material_profile. Run with UPDATE_SCHEMAS=1 to update."
            );
            assert_eq!(
                read_schema(&report_path),
                report_schema_str.replace("\r\n", "\n"),
                "Schema mismatch for validation_report. Run with UPDATE_SCHEMAS=1 to update."
            );
            assert_eq!(
                read_schema(&connection_path),
                connection_schema_str.replace("\r\n", "\n"),
                "Schema mismatch for connection_config. Run with UPDATE_SCHEMAS=1 to update."
            );
        }
    }
}

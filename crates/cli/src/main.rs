// PrintProof3D Command Line Interface
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use printproof3d_core::{PrinterProfile, MaterialProfile, ValidationReport, ValidationStatus, ModelMetadata, BuildVolume};

#[derive(Parser)]
#[command(name = "printproof3d")]
#[command(about = "CLI tool for PrintProof3D compatibility and printability engine", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a 3D model mesh against printer and material profiles
    ValidateModel {
        /// Path to the 3D model file (e.g., STL, OBJ)
        #[arg(long, short = 'm')]
        model: PathBuf,

        /// Path to the printer profile JSON file
        #[arg(long, short = 'p')]
        printer: PathBuf,

        /// Path to the material profile JSON file
        #[arg(long, short = 'a')]
        material: PathBuf,

        /// Path to write the output validation report JSON
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Validate G-code against printer and material profiles
    ValidateGcode {
        /// Path to the G-code file (e.g., .gcode)
        #[arg(long, short = 'g')]
        gcode: PathBuf,

        /// Path to the printer profile JSON file
        #[arg(long, short = 'p')]
        printer: PathBuf,

        /// Path to the material profile JSON file
        #[arg(long, short = 'a')]
        material: Option<PathBuf>,

        /// Path to write the output validation report JSON
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
}

fn read_file_to_string(path: &PathBuf) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file {:?}: {}", path, e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
    Ok(contents)
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::ValidateModel { model, printer, material, output } => {
            if !model.exists() {
                eprintln!("Error: Model file {:?} does not exist", model);
                std::process::exit(1);
            }
            let printer_json = match read_file_to_string(&printer) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let printer_profile: PrinterProfile = match serde_json::from_str(&printer_json) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: Failed to parse printer profile JSON: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = printer_profile.validate() {
                eprintln!("Error: Printer profile validation failed: {}", e);
                std::process::exit(1);
            }

            let material_json = match read_file_to_string(&material) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let material_profile: MaterialProfile = match serde_json::from_str(&material_json) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Error: Failed to parse material profile JSON: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = material_profile.validate() {
                eprintln!("Error: Material profile validation failed: {}", e);
                std::process::exit(1);
            }

            // Create a mock validation report for Stage 1 CLI logic
            let report = ValidationReport {
                status: ValidationStatus::Pass,
                target_printer_profile: format!("{}_{}", printer_profile.manufacturer, printer_profile.model),
                target_material_profile: material_profile.name.clone(),
                model: ModelMetadata {
                    file_name: model.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                    units: "mm".to_string(),
                    bounding_box: BuildVolume::Rectangular { x: 50.0, y: 50.0, z: 50.0 },
                },
                issues: vec![],
                confidence_level: "high".to_string(),
                sliced_settings_assumed: None,
            };

            let report_json = serde_json::to_string_pretty(&report).unwrap();
            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &report_json) {
                    eprintln!("Error: Failed to write report to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                println!("{}", report_json);
            }
        }
        Commands::ValidateGcode { gcode, printer, material, output } => {
            if !gcode.exists() {
                eprintln!("Error: G-code file {:?} does not exist", gcode);
                std::process::exit(1);
            }
            let printer_json = match read_file_to_string(&printer) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let printer_profile: PrinterProfile = match serde_json::from_str(&printer_json) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: Failed to parse printer profile JSON: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = printer_profile.validate() {
                eprintln!("Error: Printer profile validation failed: {}", e);
                std::process::exit(1);
            }

            let material_name = if let Some(mat_path) = material {
                let material_json = match read_file_to_string(&mat_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                };
                let material_profile: MaterialProfile = match serde_json::from_str(&material_json) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error: Failed to parse material profile JSON: {}", e);
                        std::process::exit(1);
                    }
                };
                if let Err(e) = material_profile.validate() {
                    eprintln!("Error: Material profile validation failed: {}", e);
                    std::process::exit(1);
                }
                material_profile.name
            } else {
                "none".to_string()
            };

            // Create a mock validation report for Stage 1 CLI logic
            let report = ValidationReport {
                status: ValidationStatus::Pass,
                target_printer_profile: format!("{}_{}", printer_profile.manufacturer, printer_profile.model),
                target_material_profile: material_name,
                model: ModelMetadata {
                    file_name: gcode.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                    units: "mm".to_string(),
                    bounding_box: BuildVolume::Rectangular { x: 50.0, y: 50.0, z: 50.0 },
                },
                issues: vec![],
                confidence_level: "high".to_string(),
                sliced_settings_assumed: None,
            };

            let report_json = serde_json::to_string_pretty(&report).unwrap();
            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &report_json) {
                    eprintln!("Error: Failed to write report to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                println!("{}", report_json);
            }
        }
    }
}

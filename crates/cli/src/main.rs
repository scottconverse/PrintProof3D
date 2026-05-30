// PrintProof3D Command Line Interface
pub mod mcp;
use clap::{Parser, Subcommand};
use printproof3d_core::{MaterialProfile, PrinterProfile, ValidationStatus};
use printproof3d_printability::{
    GcodeValidator, ModelValidator, StandardGcodeValidator, StlModelValidator,
};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

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
    /// Start the Model Context Protocol (MCP) JSON-RPC server on stdin/stdout
    Mcp,
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

        /// Path to the custom rules WASM validation plugin
        #[arg(long, short = 'l')]
        plugin: Option<PathBuf>,
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

        /// Path to the custom rules WASM validation plugin
        #[arg(long, short = 'l')]
        plugin: Option<PathBuf>,
    },
}

fn read_file_to_string(path: &PathBuf) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Failed to open file {:?}: {}", path, e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
    Ok(contents)
}

fn run_validation_plugin(
    plugin_path: &std::path::Path,
    report: &printproof3d_core::ValidationReport,
) -> Result<printproof3d_core::ValidationReport, String> {
    let wasm_bytes = std::fs::read(plugin_path)
        .map_err(|e| format!("Failed to read plugin file {:?}: {}", plugin_path, e))?;

    let engine = printproof3d_plugins::PluginEngine::new();
    let mut loaded = engine
        .load_plugin(&wasm_bytes)
        .map_err(|e| format!("Failed to load plugin: {}", e))?;

    let report_json =
        serde_json::to_string(report).map_err(|e| format!("Failed to serialize report: {}", e))?;

    let modified_json = loaded
        .execute_validation(&report_json)
        .map_err(|e| format!("Failed to run plugin validation: {}", e))?;

    let modified_report: printproof3d_core::ValidationReport = serde_json::from_str(&modified_json)
        .map_err(|e| format!("Failed to deserialize modified report: {}", e))?;

    Ok(modified_report)
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mcp => {
            mcp::run_mcp_server();
        }
        Commands::ValidateModel {
            model,
            printer,
            material,
            output,
            plugin,
        } => {
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

            // Run real StlModelValidator validation engine
            let validator = StlModelValidator;
            let mut report =
                match validator.validate_mesh(&model, &printer_profile, &material_profile) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error: Model validation failed: {}", e);
                        std::process::exit(1);
                    }
                };

            if let Some(plugin_path) = plugin {
                report = match run_validation_plugin(&plugin_path, &report) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error: Plugin execution failed: {}", e);
                        std::process::exit(1);
                    }
                };
            }

            let has_warnings_or_failures = report.status == ValidationStatus::Warning
                || report.status == ValidationStatus::Fail;
            let report_json = serde_json::to_string_pretty(&report).unwrap();
            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &report_json) {
                    eprintln!("Error: Failed to write report to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                println!("{}", report_json);
            }
            if has_warnings_or_failures {
                std::process::exit(1);
            }
        }
        Commands::ValidateGcode {
            gcode,
            printer,
            material,
            output,
            plugin,
        } => {
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

            let material_profile = if let Some(mat_path) = material {
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
                material_profile
            } else {
                MaterialProfile {
                    name: "Generic".to_string(),
                    abbreviations: vec![],
                    min_nozzle_temp: 0.0,
                    max_nozzle_temp: 500.0,
                    min_bed_temp: 0.0,
                    max_bed_temp: 200.0,
                    cooling_fan_speed_pct: 0.0,
                    warp_risk: printproof3d_core::RiskLevel::Low,
                    bridge_difficulty: printproof3d_core::RiskLevel::Low,
                    overhang_difficulty: printproof3d_core::RiskLevel::Low,
                    enclosure_recommended: false,
                    dryness_sensitive: false,
                    bed_adhesion_notes: None,
                    min_feature_size_mm: 0.4,
                }
            };

            // Run real StandardGcodeValidator validation engine
            let validator = StandardGcodeValidator;
            let mut report =
                match validator.validate_gcode(&gcode, &printer_profile, &material_profile) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error: G-code validation failed: {}", e);
                        std::process::exit(1);
                    }
                };

            if let Some(plugin_path) = plugin {
                report = match run_validation_plugin(&plugin_path, &report) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error: Plugin execution failed: {}", e);
                        std::process::exit(1);
                    }
                };
            }

            let has_warnings_or_failures = report.status == ValidationStatus::Warning
                || report.status == ValidationStatus::Fail;
            let report_json = serde_json::to_string_pretty(&report).unwrap();
            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &report_json) {
                    eprintln!("Error: Failed to write report to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                println!("{}", report_json);
            }
            if has_warnings_or_failures {
                std::process::exit(1);
            }
        }
    }
}

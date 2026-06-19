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
    /// Preflight a print job (validate model/G-code and optionally run simulator twin check)
    Preflight {
        /// Path to the 3D model file (e.g., STL)
        #[arg(long, short = 'm')]
        model: Option<PathBuf>,

        /// Path to the G-code file (e.g., .gcode)
        #[arg(long, short = 'g')]
        gcode: Option<PathBuf>,

        /// Path to the printer profile JSON file
        #[arg(long, short = 'p')]
        printer: PathBuf,

        /// Path to the material profile JSON file
        #[arg(long, short = 'a')]
        material: Option<PathBuf>,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "json")]
        format: String,

        /// Path to write the output validation report
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        /// Path to the custom rules WASM validation plugin
        #[arg(long, short = 'l')]
        plugin: Option<PathBuf>,

        /// Enable simulator check for a specific protocol (rrf, octoprint, moonraker, prusalink, bambu, serial)
        #[arg(long, short = 's')]
        simulator: Option<String>,
    },
    /// List available printer profiles in a directory
    ListPrinters {
        /// Directory containing printer profiles
        #[arg(long, short = 'd')]
        directory: Option<PathBuf>,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,
    },
    /// List available material profiles in a directory
    ListMaterials {
        /// Directory containing material profiles
        #[arg(long, short = 'd')]
        directory: Option<PathBuf>,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,
    },
    /// Inspect a printer or material profile detailing its fields
    InspectProfile {
        /// Path to the profile JSON file
        path: PathBuf,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,

        /// Path to write the output
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Validate a printer profile JSON file against safety invariants
    ValidatePrinterProfile {
        /// Path to the printer profile JSON file
        path: PathBuf,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,

        /// Path to write the output
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Validate a material profile JSON file against safety invariants
    ValidateMaterialProfile {
        /// Path to the material profile JSON file
        path: PathBuf,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,

        /// Path to write the output
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Perform compatibility checks between profiles and files
    CheckCompatibility {
        /// Path to the target printer profile JSON file
        #[arg(long, short = 'p')]
        printer: PathBuf,

        /// Path to the material profile JSON file
        #[arg(long, short = 'a')]
        material: Option<PathBuf>,

        /// Path to the 3D model file (e.g., STL)
        #[arg(long, short = 'm')]
        model: Option<PathBuf>,

        /// Path to the G-code file (e.g., .gcode)
        #[arg(long, short = 'g')]
        gcode: Option<PathBuf>,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,

        /// Path to write the output
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Generate a default template printer profile JSON file
    GeneratePrinterProfile {
        /// Path to write the output profile JSON
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Generate a default template material profile JSON file
    GenerateMaterialProfile {
        /// Path to write the output profile JSON
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Validate all profile JSON files inside a directory
    ValidateProfileDirectory {
        /// Directory containing profiles to validate
        #[arg(value_name = "DIRECTORY")]
        directory: Option<PathBuf>,

        /// Directory containing profiles to validate via option
        #[arg(long = "directory", short = 'd', conflicts_with = "directory")]
        opt_directory: Option<PathBuf>,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,

        /// Path to write the validation results
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
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

    let mut modified_report: printproof3d_core::ValidationReport =
        serde_json::from_str(&modified_json)
            .map_err(|e| format!("Failed to deserialize modified report: {}", e))?;

    modified_report.enforce_invariants();

    Ok(modified_report)
}

async fn run_simulator_preflight_check(
    profile: &PrinterProfile,
    protocol: &str,
) -> Result<serde_json::Value, String> {
    use printproof3d_adapters::factory::PrinterAdapterFactory;
    use printproof3d_core::connection::{
        AuthType, ConnectionMode, DispatchPolicy, PrinterConnectionConfig,
    };
    use printproof3d_core::ProtocolFamily;

    match protocol {
        "rrf" => {
            let server = printproof3d_sdk::mocks::RrfMockServer::start();
            let config = PrinterConnectionConfig {
                name: "RRF Preflight Simulator".to_string(),
                mode: ConnectionMode::Simulator,
                protocol_family: ProtocolFamily::RepRapFirmware,
                base_url: Some(format!("http://127.0.0.1:{}", server.port)),
                serial_path: None,
                serial_baud_rate: None,
                auth_type: AuthType::None,
                api_key_env_var: None,
                username: None,
                password_env_var: None,
                tls_enabled: false,
                dispatch_policy: DispatchPolicy::AllowStart,
                simulator_scenario: None,
            };
            let mut adapter = PrinterAdapterFactory::build(profile, &config)
                .map_err(|e: printproof3d_adapters::AdapterError| format!("Adapter build error: {:?}", e))?;

            let adapter_ref: &mut dyn printproof3d_adapters::PrinterAdapter = adapter.as_mut();
            adapter_ref.connect().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Connect error: {:?}", e))?;
            let telemetry = adapter_ref.get_status().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Get status error: {:?}", e))?;
            let _ = adapter_ref.disconnect().await;

            server.stop();

            Ok(serde_json::json!({
                "protocol": "rep_rap_firmware",
                "state": format!("{:?}", telemetry.state),
                "tool_temp": telemetry.tool_temp,
                "tool_target": telemetry.tool_target,
                "bed_temp": telemetry.bed_temp,
                "bed_target": telemetry.bed_target,
            }))
        }
        "octoprint" => {
            let mut server = printproof3d_sdk::mocks::OctoPrintMockServer::start();
            let config = PrinterConnectionConfig {
                name: "OctoPrint Preflight Simulator".to_string(),
                mode: ConnectionMode::Simulator,
                protocol_family: ProtocolFamily::OctoPrint,
                base_url: Some(server.get_url()),
                serial_path: None,
                serial_baud_rate: None,
                auth_type: AuthType::ApiKey,
                api_key_env_var: Some("OCTOPRINT_API_KEY".to_string()),
                username: None,
                password_env_var: None,
                tls_enabled: false,
                dispatch_policy: DispatchPolicy::AllowStart,
                simulator_scenario: None,
            };
            std::env::set_var("OCTOPRINT_API_KEY", "secret_key");
            let mut adapter = PrinterAdapterFactory::build(profile, &config)
                .map_err(|e: printproof3d_adapters::AdapterError| format!("Adapter build error: {:?}", e))?;

            let adapter_ref: &mut dyn printproof3d_adapters::PrinterAdapter = adapter.as_mut();
            adapter_ref.connect().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Connect error: {:?}", e))?;
            let telemetry = adapter_ref.get_status().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Get status error: {:?}", e))?;
            let _ = adapter_ref.disconnect().await;

            server.stop();

            Ok(serde_json::json!({
                "protocol": "octoprint",
                "state": format!("{:?}", telemetry.state),
                "tool_temp": telemetry.tool_temp,
                "tool_target": telemetry.tool_target,
                "bed_temp": telemetry.bed_temp,
                "bed_target": telemetry.bed_target,
            }))
        }
        "moonraker" => {
            let mut server = printproof3d_sdk::mocks::MoonrakerMockServer::start();
            let config = PrinterConnectionConfig {
                name: "Moonraker Preflight Simulator".to_string(),
                mode: ConnectionMode::Simulator,
                protocol_family: ProtocolFamily::Klipper,
                base_url: Some(server.get_url()),
                serial_path: None,
                serial_baud_rate: None,
                auth_type: AuthType::None,
                api_key_env_var: None,
                username: None,
                password_env_var: None,
                tls_enabled: false,
                dispatch_policy: DispatchPolicy::AllowStart,
                simulator_scenario: None,
            };
            let mut adapter = PrinterAdapterFactory::build(profile, &config)
                .map_err(|e: printproof3d_adapters::AdapterError| format!("Adapter build error: {:?}", e))?;

            let adapter_ref: &mut dyn printproof3d_adapters::PrinterAdapter = adapter.as_mut();
            adapter_ref.connect().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Connect error: {:?}", e))?;
            let telemetry = adapter_ref.get_status().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Get status error: {:?}", e))?;
            let _ = adapter_ref.disconnect().await;

            server.stop();

            Ok(serde_json::json!({
                "protocol": "klipper",
                "state": format!("{:?}", telemetry.state),
                "tool_temp": telemetry.tool_temp,
                "tool_target": telemetry.tool_target,
                "bed_temp": telemetry.bed_temp,
                "bed_target": telemetry.bed_target,
            }))
        }
        "prusalink" => {
            let mut server = printproof3d_sdk::mocks::PrusaLinkMockServer::start();
            let config = PrinterConnectionConfig {
                name: "PrusaLink Preflight Simulator".to_string(),
                mode: ConnectionMode::Simulator,
                protocol_family: ProtocolFamily::PrusaLink,
                base_url: Some(server.get_url()),
                serial_path: None,
                serial_baud_rate: None,
                auth_type: AuthType::Digest,
                api_key_env_var: None,
                username: Some("maker".to_string()),
                password_env_var: Some("PRUSALINK_PASSWORD".to_string()),
                tls_enabled: false,
                dispatch_policy: DispatchPolicy::AllowStart,
                simulator_scenario: None,
            };
            std::env::set_var("PRUSALINK_PASSWORD", "makerpass");
            let mut adapter = PrinterAdapterFactory::build(profile, &config)
                .map_err(|e: printproof3d_adapters::AdapterError| format!("Adapter build error: {:?}", e))?;

            let adapter_ref: &mut dyn printproof3d_adapters::PrinterAdapter = adapter.as_mut();
            adapter_ref.connect().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Connect error: {:?}", e))?;
            let telemetry = adapter_ref.get_status().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Get status error: {:?}", e))?;
            let _ = adapter_ref.disconnect().await;

            server.stop();

            Ok(serde_json::json!({
                "protocol": "prusalink",
                "state": format!("{:?}", telemetry.state),
                "tool_temp": telemetry.tool_temp,
                "tool_target": telemetry.tool_target,
                "bed_temp": telemetry.bed_temp,
                "bed_target": telemetry.bed_target,
            }))
        }
        "bambu" => {
            let mqtt_server = printproof3d_sdk::mocks::BambuMqttMock::start();
            let ftp_server = printproof3d_sdk::mocks::BambuFtpMock::start();
            let config = PrinterConnectionConfig {
                name: "Bambu Preflight Simulator".to_string(),
                mode: ConnectionMode::Simulator,
                protocol_family: ProtocolFamily::BambuMqtt,
                base_url: Some(format!("127.0.0.1:{}:{}", mqtt_server.get_port(), ftp_server.get_port())),
                serial_path: None,
                serial_baud_rate: None,
                auth_type: AuthType::None,
                api_key_env_var: None,
                username: None,
                password_env_var: None,
                tls_enabled: false,
                dispatch_policy: DispatchPolicy::AllowStart,
                simulator_scenario: None,
            };
            let mut adapter = PrinterAdapterFactory::build(profile, &config)
                .map_err(|e: printproof3d_adapters::AdapterError| format!("Adapter build error: {:?}", e))?;

            let adapter_ref: &mut dyn printproof3d_adapters::PrinterAdapter = adapter.as_mut();
            adapter_ref.connect().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Connect error: {:?}", e))?;
            tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
            let telemetry = adapter_ref.get_status().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Get status error: {:?}", e))?;
            let _ = adapter_ref.disconnect().await;

            mqtt_server.stop();
            ftp_server.stop();

            Ok(serde_json::json!({
                "protocol": "bambu_mqtt",
                "state": format!("{:?}", telemetry.state),
                "tool_temp": telemetry.tool_temp,
                "tool_target": telemetry.tool_target,
                "bed_temp": telemetry.bed_temp,
                "bed_target": telemetry.bed_target,
            }))
        }
        "serial" => {
            let config = PrinterConnectionConfig {
                name: "Marlin Serial Preflight Simulator".to_string(),
                mode: ConnectionMode::Simulator,
                protocol_family: ProtocolFamily::MarlinSerial,
                base_url: None,
                serial_path: Some("COM3".to_string()),
                serial_baud_rate: Some(115200),
                auth_type: AuthType::None,
                api_key_env_var: None,
                username: None,
                password_env_var: None,
                tls_enabled: false,
                dispatch_policy: DispatchPolicy::AllowStart,
                simulator_scenario: None,
            };
            let mut adapter = PrinterAdapterFactory::build(profile, &config)
                .map_err(|e: printproof3d_adapters::AdapterError| format!("Adapter build error: {:?}", e))?;

            let adapter_ref: &mut dyn printproof3d_adapters::PrinterAdapter = adapter.as_mut();
            adapter_ref.connect().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Connect error: {:?}", e))?;
            let telemetry = adapter_ref.get_status().await.map_err(|e: printproof3d_adapters::AdapterError| format!("Get status error: {:?}", e))?;
            let _ = adapter_ref.disconnect().await;

            Ok(serde_json::json!({
                "protocol": "marlin_serial",
                "state": format!("{:?}", telemetry.state),
                "tool_temp": telemetry.tool_temp,
                "tool_target": telemetry.tool_target,
                "bed_temp": telemetry.bed_temp,
                "bed_target": telemetry.bed_target,
            }))
        }
        _ => Err(format!(
            "Unsupported simulator protocol: {}. Supported: rrf, octoprint, moonraker, prusalink, bambu, serial",
            protocol
        )),
    }
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
        Commands::Preflight {
            model,
            gcode,
            printer,
            material,
            format,
            output,
            plugin,
            simulator,
        } => {
            if model.is_some() && gcode.is_some() {
                eprintln!(
                    "Error: Cannot provide both --model and --gcode. Choose one validation target."
                );
                std::process::exit(1);
            }
            if model.is_none() && gcode.is_none() {
                eprintln!("Error: Must provide either --model or --gcode to validate.");
                std::process::exit(1);
            }

            if model.is_some() && material.is_none() {
                eprintln!(
                    "Error: --material profile is required when validating a 3D model (--model)."
                );
                std::process::exit(1);
            }

            if let Some(ref m_path) = model {
                if !m_path.exists() {
                    eprintln!("Error: Model file {:?} does not exist", m_path);
                    std::process::exit(1);
                }
            }
            if let Some(ref g_path) = gcode {
                if !g_path.exists() {
                    eprintln!("Error: G-code file {:?} does not exist", g_path);
                    std::process::exit(1);
                }
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

            let material_profile = if let Some(ref mat_path) = material {
                let material_json = match read_file_to_string(mat_path) {
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

            let mut report = if let Some(ref m_path) = model {
                let validator = StlModelValidator;
                match validator.validate_mesh(m_path, &printer_profile, &material_profile) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error: Model validation failed: {}", e);
                        std::process::exit(1);
                    }
                }
            } else if let Some(ref g_path) = gcode {
                let validator = StandardGcodeValidator;
                match validator.validate_gcode(g_path, &printer_profile, &material_profile) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error: G-code validation failed: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                unreachable!()
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

            if let Some(sim_proto) = simulator {
                let sim_lower = sim_proto.to_lowercase();
                let rt = tokio::runtime::Runtime::new().unwrap();
                let check_result = rt.block_on(async {
                    run_simulator_preflight_check(&printer_profile, &sim_lower).await
                });

                match check_result {
                    Ok(telemetry) => {
                        let mut map = match report.sliced_settings_assumed.take() {
                            Some(serde_json::Value::Object(m)) => m,
                            _ => serde_json::Map::new(),
                        };
                        map.insert("simulator_telemetry".to_string(), telemetry);
                        report.sliced_settings_assumed = Some(serde_json::Value::Object(map));
                    }
                    Err(e) => {
                        report.issues.push(printproof3d_core::ValidationIssue {
                            id: "PRINTER_CONNECTION_FAILED".to_string(),
                            severity: printproof3d_core::IssueSeverity::Critical,
                            message: format!("Simulator connection check failed: {}", e),
                            location: None,
                            suggested_fixes: vec![
                                "Verify that the mock server or simulator twin is configured correctly.".to_string(),
                                "Check configuration protocol family constraints.".to_string()
                            ],
                        });
                        report.status = printproof3d_core::ValidationStatus::Fail;
                    }
                }
            }

            let has_warnings_or_failures = report.status == ValidationStatus::Warning
                || report.status == ValidationStatus::Fail;

            let fmt_lower = format.to_lowercase();
            if fmt_lower != "text" && fmt_lower != "json" {
                eprintln!(
                    "Error: Unsupported format '{}'. Supported formats: text, json",
                    format
                );
                std::process::exit(1);
            }

            let output_str = if fmt_lower == "json" {
                serde_json::to_string_pretty(&report).unwrap()
            } else {
                format_preflight_text(&report)
            };

            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &output_str) {
                    eprintln!("Error: Failed to write report to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                println!("{}", output_str);
            }

            if has_warnings_or_failures {
                std::process::exit(1);
            }
        }
        Commands::ListPrinters { directory, format } => {
            let dir = directory.unwrap_or_else(|| PathBuf::from("profiles"));
            let fmt_lower = format.to_lowercase();
            if fmt_lower != "text" && fmt_lower != "json" {
                eprintln!(
                    "Error: Unsupported format '{}'. Supported formats: text, json",
                    format
                );
                std::process::exit(1);
            }
            let is_json = fmt_lower == "json";

            let mut profiles = list_profiles_in_dir::<PrinterProfile>(&dir);
            profiles.sort_by(|a, b| {
                let a_key = format!("{}_{}", a.1.manufacturer, a.1.model);
                let b_key = format!("{}_{}", b.1.manufacturer, b.1.model);
                a_key.cmp(&b_key)
            });

            if is_json {
                let output_array: Vec<serde_json::Value> = profiles
                    .iter()
                    .map(|(p, prof)| {
                        serde_json::json!({
                            "file": p.to_string_lossy(),
                            "manufacturer": prof.manufacturer,
                            "model": prof.model,
                            "protocol_family": serde_json::to_value(&prof.protocol_family).unwrap(),
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&output_array).unwrap());
            } else {
                if profiles.is_empty() {
                    println!("No printer profiles found in directory {:?}", dir);
                } else {
                    println!("Printer Profiles found in {:?}:", dir);
                    for (p, prof) in &profiles {
                        println!(
                            "  - Manufacturer: {}, Model: {}, Protocol: {:?} (File: {:?})",
                            prof.manufacturer, prof.model, prof.protocol_family, p
                        );
                    }
                }
            }
        }
        Commands::ListMaterials { directory, format } => {
            let dir = directory.unwrap_or_else(|| PathBuf::from("profiles"));
            let fmt_lower = format.to_lowercase();
            if fmt_lower != "text" && fmt_lower != "json" {
                eprintln!(
                    "Error: Unsupported format '{}'. Supported formats: text, json",
                    format
                );
                std::process::exit(1);
            }
            let is_json = fmt_lower == "json";

            let mut profiles = list_profiles_in_dir::<MaterialProfile>(&dir);
            profiles.sort_by(|a, b| a.1.name.cmp(&b.1.name));

            if is_json {
                let output_array: Vec<serde_json::Value> = profiles
                    .iter()
                    .map(|(p, prof)| {
                        serde_json::json!({
                            "file": p.to_string_lossy(),
                            "name": prof.name,
                            "abbreviations": prof.abbreviations,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&output_array).unwrap());
            } else {
                if profiles.is_empty() {
                    println!("No material profiles found in directory {:?}", dir);
                } else {
                    println!("Material Profiles found in {:?}:", dir);
                    for (p, prof) in &profiles {
                        println!(
                            "  - Name: {}, Abbreviations: {:?} (File: {:?})",
                            prof.name, prof.abbreviations, p
                        );
                    }
                }
            }
        }
        Commands::InspectProfile {
            path,
            format,
            output,
        } => {
            let fmt_lower = format.to_lowercase();
            if fmt_lower != "text" && fmt_lower != "json" {
                eprintln!(
                    "Error: Unsupported format '{}'. Supported formats: text, json",
                    format
                );
                std::process::exit(1);
            }
            let is_json = fmt_lower == "json";
            let content = match read_file_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: Failed to read profile file: {}", e);
                    std::process::exit(1);
                }
            };

            let mut output_str = String::new();
            if let Ok(printer) = serde_json::from_str::<PrinterProfile>(&content) {
                if is_json {
                    let wrapped = serde_json::json!({
                        "profile_type": "printer",
                        "profile": printer,
                    });
                    output_str = serde_json::to_string_pretty(&wrapped).unwrap() + "\n";
                } else {
                    use std::fmt::Write;
                    writeln!(output_str, "Profile Type    : Printer Profile").unwrap();
                    writeln!(output_str, "File Path       : {:?}", path).unwrap();
                    writeln!(output_str, "Manufacturer    : {}", printer.manufacturer).unwrap();
                    writeln!(output_str, "Model           : {}", printer.model).unwrap();
                    writeln!(
                        output_str,
                        "Protocol Family : {:?}",
                        printer.protocol_family
                    )
                    .unwrap();
                    writeln!(output_str, "Build Volume    : {:?}", printer.build_volume).unwrap();
                    writeln!(output_str, "Bed Shape       : {:?}", printer.bed_shape).unwrap();
                    writeln!(
                        output_str,
                        "Nozzle Diameters: {:?}",
                        printer.nozzle_diameters
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Default Nozzle  : {:.2} mm",
                        printer.default_nozzle_diameter
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Max Hotend Temp : {:.1}°C",
                        printer.max_hotend_temp
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Max Bed Temp    : {:.1}°C",
                        printer.max_bed_temp
                    )
                    .unwrap();
                    writeln!(output_str, "Has Enclosure   : {}", printer.has_enclosure).unwrap();
                    writeln!(output_str, "Supports MMU    : {}", printer.supports_mmu).unwrap();
                    writeln!(
                        output_str,
                        "Firmware Flavor : {:?}",
                        printer.firmware_flavor
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "File Types      : {:?}",
                        printer.supported_file_types
                    )
                    .unwrap();
                }
            } else if let Ok(material) = serde_json::from_str::<MaterialProfile>(&content) {
                if is_json {
                    let wrapped = serde_json::json!({
                        "profile_type": "material",
                        "profile": material,
                    });
                    output_str = serde_json::to_string_pretty(&wrapped).unwrap() + "\n";
                } else {
                    use std::fmt::Write;
                    writeln!(output_str, "Profile Type    : Material Profile").unwrap();
                    writeln!(output_str, "File Path       : {:?}", path).unwrap();
                    writeln!(output_str, "Name            : {}", material.name).unwrap();
                    writeln!(output_str, "Abbreviations   : {:?}", material.abbreviations).unwrap();
                    writeln!(
                        output_str,
                        "Min Nozzle Temp : {:.1}°C",
                        material.min_nozzle_temp
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Max Nozzle Temp : {:.1}°C",
                        material.max_nozzle_temp
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Min Bed Temp    : {:.1}°C",
                        material.min_bed_temp
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Max Bed Temp    : {:.1}°C",
                        material.max_bed_temp
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Fan Speed Pct   : {:.1}%",
                        material.cooling_fan_speed_pct
                    )
                    .unwrap();
                    writeln!(output_str, "Warp Risk       : {:?}", material.warp_risk).unwrap();
                    writeln!(
                        output_str,
                        "Bridge Diff     : {:?}",
                        material.bridge_difficulty
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Overhang Diff   : {:?}",
                        material.overhang_difficulty
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Enclosure Rec   : {}",
                        material.enclosure_recommended
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Dryness Sens.   : {}",
                        material.dryness_sensitive
                    )
                    .unwrap();
                    writeln!(
                        output_str,
                        "Min Feature Size: {:.2} mm",
                        material.min_feature_size_mm
                    )
                    .unwrap();
                }
            } else {
                eprintln!(
                    "Error: Structurally invalid profile or malformed JSON file at {:?}",
                    path
                );
                std::process::exit(1);
            }

            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &output_str) {
                    eprintln!("Error: Failed to write output to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                print!("{}", output_str);
            }
        }
        Commands::ValidatePrinterProfile {
            path,
            format,
            output,
        } => {
            let fmt_lower = format.to_lowercase();
            if fmt_lower != "text" && fmt_lower != "json" {
                eprintln!(
                    "Error: Unsupported format '{}'. Supported formats: text, json",
                    format
                );
                std::process::exit(1);
            }
            let is_json = fmt_lower == "json";
            let content = match read_file_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: Failed to read profile file: {}", e);
                    std::process::exit(1);
                }
            };
            let parsed: Result<PrinterProfile, _> = serde_json::from_str(&content);
            let mut is_ok = false;
            let output_str = match parsed {
                Ok(printer) => match printer.validate() {
                    Ok(()) => {
                        is_ok = true;
                        if is_json {
                            serde_json::json!({ "valid": true, "type": "printer", "file": path.to_string_lossy() }).to_string() + "\n"
                        } else {
                            format!("Printer profile {:?} is valid.\n", path)
                        }
                    }
                    Err(e) => {
                        if is_json {
                            serde_json::json!({ "valid": false, "type": "printer", "file": path.to_string_lossy(), "error": e }).to_string() + "\n"
                        } else {
                            format!("Error: Printer profile validation failed: {}\n", e)
                        }
                    }
                },
                Err(e) => {
                    if is_json {
                        serde_json::json!({ "valid": false, "type": "printer", "file": path.to_string_lossy(), "error": e.to_string() }).to_string() + "\n"
                    } else {
                        format!("Error: Failed to parse printer profile JSON: {}\n", e)
                    }
                }
            };

            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &output_str) {
                    eprintln!("Error: Failed to write output to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else if !is_ok && !is_json {
                eprint!("{}", output_str);
            } else {
                print!("{}", output_str);
            }

            if !is_ok {
                std::process::exit(1);
            }
        }
        Commands::ValidateMaterialProfile {
            path,
            format,
            output,
        } => {
            let fmt_lower = format.to_lowercase();
            if fmt_lower != "text" && fmt_lower != "json" {
                eprintln!(
                    "Error: Unsupported format '{}'. Supported formats: text, json",
                    format
                );
                std::process::exit(1);
            }
            let is_json = fmt_lower == "json";
            let content = match read_file_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: Failed to read profile file: {}", e);
                    std::process::exit(1);
                }
            };
            let parsed: Result<MaterialProfile, _> = serde_json::from_str(&content);
            let mut is_ok = false;
            let output_str = match parsed {
                Ok(material) => match material.validate() {
                    Ok(()) => {
                        is_ok = true;
                        if is_json {
                            serde_json::json!({ "valid": true, "type": "material", "file": path.to_string_lossy() }).to_string() + "\n"
                        } else {
                            format!("Material profile {:?} is valid.\n", path)
                        }
                    }
                    Err(e) => {
                        if is_json {
                            serde_json::json!({ "valid": false, "type": "material", "file": path.to_string_lossy(), "error": e }).to_string() + "\n"
                        } else {
                            format!("Error: Material profile validation failed: {}\n", e)
                        }
                    }
                },
                Err(e) => {
                    if is_json {
                        serde_json::json!({ "valid": false, "type": "material", "file": path.to_string_lossy(), "error": e.to_string() }).to_string() + "\n"
                    } else {
                        format!("Error: Failed to parse material profile JSON: {}\n", e)
                    }
                }
            };

            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &output_str) {
                    eprintln!("Error: Failed to write output to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else if !is_ok && !is_json {
                eprint!("{}", output_str);
            } else {
                print!("{}", output_str);
            }

            if !is_ok {
                std::process::exit(1);
            }
        }
        Commands::CheckCompatibility {
            printer,
            material,
            model,
            gcode,
            format,
            output,
        } => {
            let fmt_lower = format.to_lowercase();
            if fmt_lower != "text" && fmt_lower != "json" {
                eprintln!(
                    "Error: Unsupported format '{}'. Supported formats: text, json",
                    format
                );
                std::process::exit(1);
            }
            if model.is_some() && gcode.is_some() {
                eprintln!("Error: Cannot provide both --model and --gcode.");
                std::process::exit(1);
            }
            if material.is_none() && model.is_none() && gcode.is_none() {
                eprintln!("Error: Must provide at least one of --material, --model, or --gcode to check compatibility.");
                std::process::exit(1);
            }

            let printer_json = match read_file_to_string(&printer) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: Failed to read printer profile: {}", e);
                    std::process::exit(1);
                }
            };
            let printer_profile: PrinterProfile = match serde_json::from_str(&printer_json) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: Failed to parse printer profile: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = printer_profile.validate() {
                eprintln!("Error: Printer profile validation failed: {}", e);
                std::process::exit(1);
            }

            let mut issues = Vec::new();
            let mut resolved_material = None;

            if let Some(ref mat_path) = material {
                let material_json = match read_file_to_string(mat_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error: Failed to read material profile: {}", e);
                        std::process::exit(1);
                    }
                };
                let material_profile: MaterialProfile = match serde_json::from_str(&material_json) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error: Failed to parse material profile: {}", e);
                        std::process::exit(1);
                    }
                };
                if let Err(e) = material_profile.validate() {
                    eprintln!("Error: Material profile validation failed: {}", e);
                    std::process::exit(1);
                }

                // 1. Run printer-material compatibility checks
                let mat_issues =
                    printproof3d_printability::compatibility::check_printer_material_compatibility(
                        &printer_profile,
                        &material_profile,
                    );
                issues.extend(mat_issues);
                resolved_material = Some(material_profile);
            }

            // 2. Run model/gcode printability checks if provided
            let mat_ref = resolved_material
                .as_ref()
                .cloned()
                .unwrap_or_else(|| MaterialProfile {
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
                });

            if let Some(ref m_path) = model {
                let validator = StlModelValidator;
                match validator.validate_mesh(m_path, &printer_profile, &mat_ref) {
                    Ok(report) => {
                        issues.extend(report.issues);
                    }
                    Err(e) => {
                        eprintln!("Error: Model validation failed: {}", e);
                        std::process::exit(1);
                    }
                }
            } else if let Some(ref g_path) = gcode {
                let validator = StandardGcodeValidator;
                match validator.validate_gcode(g_path, &printer_profile, &mat_ref) {
                    Ok(report) => {
                        issues.extend(report.issues);
                    }
                    Err(e) => {
                        eprintln!("Error: G-code validation failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            let mut status = ValidationStatus::Pass;
            for issue in &issues {
                match issue.severity {
                    printproof3d_core::IssueSeverity::Blocker
                    | printproof3d_core::IssueSeverity::Critical => {
                        status = ValidationStatus::Fail;
                        break;
                    }
                    printproof3d_core::IssueSeverity::Major if status != ValidationStatus::Fail => {
                        status = ValidationStatus::Warning;
                    }
                    _ => {}
                }
            }

            let is_json = fmt_lower == "json";
            let mut output_str = String::new();
            if is_json {
                let report = serde_json::json!({
                    "status": format!("{:?}", status).to_lowercase(),
                    "printer": format!("{}_{}", printer_profile.manufacturer, printer_profile.model),
                    "material": resolved_material.map(|m| m.name),
                    "model": model.map(|p| p.file_name().unwrap().to_string_lossy().into_owned()),
                    "gcode": gcode.map(|p| p.file_name().unwrap().to_string_lossy().into_owned()),
                    "issues": issues,
                });
                output_str = serde_json::to_string_pretty(&report).unwrap() + "\n";
            } else {
                use std::fmt::Write;
                writeln!(
                    output_str,
                    "============================================================"
                )
                .unwrap();
                writeln!(output_str, "PRINTPROOF3D COMPATIBILITY REPORT").unwrap();
                writeln!(
                    output_str,
                    "============================================================"
                )
                .unwrap();
                writeln!(
                    output_str,
                    "Printer  : {} {}",
                    printer_profile.manufacturer, printer_profile.model
                )
                .unwrap();
                if let Some(ref m) = resolved_material {
                    writeln!(output_str, "Material : {}", m.name).unwrap();
                }
                if let Some(ref m_path) = model {
                    writeln!(output_str, "Model    : {:?}", m_path.file_name().unwrap()).unwrap();
                }
                if let Some(ref g_path) = gcode {
                    writeln!(output_str, "G-code   : {:?}", g_path.file_name().unwrap()).unwrap();
                }
                writeln!(output_str, "Status   : {:?}", status).unwrap();
                writeln!(
                    output_str,
                    "------------------------------------------------------------"
                )
                .unwrap();
                if issues.is_empty() {
                    writeln!(output_str, "No compatibility issues detected.").unwrap();
                    writeln!(
                        output_str,
                        "Target passes PrintProof3D profile and file validation checks."
                    )
                    .unwrap();
                } else {
                    writeln!(output_str, "Issues detected ({}):", issues.len()).unwrap();
                    for (i, issue) in issues.iter().enumerate() {
                        writeln!(
                            output_str,
                            "\n  {}. [{:?}] ID: {}",
                            i + 1,
                            issue.severity,
                            issue.id
                        )
                        .unwrap();
                        writeln!(output_str, "     Message: {}", issue.message).unwrap();
                        if !issue.suggested_fixes.is_empty() {
                            writeln!(output_str, "     Suggested Fixes:").unwrap();
                            for fix in &issue.suggested_fixes {
                                writeln!(output_str, "       - {}", fix).unwrap();
                            }
                        }
                    }
                }
                writeln!(
                    output_str,
                    "============================================================"
                )
                .unwrap();
            }

            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &output_str) {
                    eprintln!("Error: Failed to write output to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                print!("{}", output_str);
            }

            if status == ValidationStatus::Fail {
                std::process::exit(1);
            }
        }
        Commands::GeneratePrinterProfile { output } => {
            let template = PrinterProfile {
                manufacturer: "TemplateManufacturer".to_string(),
                model: "TemplateModel".to_string(),
                protocol_family: printproof3d_core::ProtocolFamily::OctoPrint,
                build_volume: printproof3d_core::BuildVolume::Rectangular {
                    x: 220.0,
                    y: 220.0,
                    z: 250.0,
                },
                bed_shape: printproof3d_core::BedShape::Rectangular,
                nozzle_diameters: vec![0.4],
                default_nozzle_diameter: 0.4,
                min_layer_height: 0.05,
                max_layer_height: 0.3,
                max_hotend_temp: 260.0,
                max_bed_temp: 100.0,
                has_enclosure: false,
                supports_mmu: false,
                firmware_flavor: printproof3d_core::FirmwareFlavor::Marlin,
                supported_file_types: vec!["gcode".to_string()],
                supports_direct_upload: true,
                supports_pause_resume: true,
                supports_cancel: true,
                supports_job_progress: true,
                supports_webcam: false,
                supports_chamber_temp: false,
                known_quirks: vec![],
                unsafe_commands: vec!["M500".to_string()],
                filename_restrictions: None,
            };
            let template_json = serde_json::to_string_pretty(&template).unwrap() + "\n";
            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &template_json) {
                    eprintln!("Error: Failed to write template to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                print!("{}", template_json);
            }
        }
        Commands::GenerateMaterialProfile { output } => {
            let template = MaterialProfile {
                name: "Template PLA".to_string(),
                abbreviations: vec!["PLA".to_string()],
                min_nozzle_temp: 190.0,
                max_nozzle_temp: 220.0,
                min_bed_temp: 50.0,
                max_bed_temp: 60.0,
                cooling_fan_speed_pct: 100.0,
                warp_risk: printproof3d_core::RiskLevel::Low,
                bridge_difficulty: printproof3d_core::RiskLevel::Low,
                overhang_difficulty: printproof3d_core::RiskLevel::Low,
                enclosure_recommended: false,
                dryness_sensitive: false,
                bed_adhesion_notes: Some("Clean PEI surface.".to_string()),
                min_feature_size_mm: 0.4,
            };
            let template_json = serde_json::to_string_pretty(&template).unwrap() + "\n";
            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &template_json) {
                    eprintln!("Error: Failed to write template to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                print!("{}", template_json);
            }
        }
        Commands::ValidateProfileDirectory {
            directory,
            opt_directory,
            format,
            output,
        } => {
            let dir = directory
                .or(opt_directory)
                .unwrap_or_else(|| PathBuf::from("profiles"));
            if !dir.exists() || !dir.is_dir() {
                eprintln!(
                    "Error: Directory {:?} does not exist or is not a directory",
                    dir
                );
                std::process::exit(1);
            }
            let fmt_lower = format.to_lowercase();
            if fmt_lower != "text" && fmt_lower != "json" {
                eprintln!(
                    "Error: Unsupported format '{}'. Supported formats: text, json",
                    format
                );
                std::process::exit(1);
            }
            let is_json = fmt_lower == "json";

            let mut all_valid = true;
            let mut results = Vec::new();
            let mut text_lines = Vec::new();

            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Error: Failed to read directory {:?}: {}", dir, e);
                    std::process::exit(1);
                }
            };

            for entry_opt in entries {
                let entry = match entry_opt {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                    let content = match std::fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(_) => {
                            all_valid = false;
                            results.push(serde_json::json!({
                                "file": path.to_string_lossy(),
                                "valid": false,
                                "type": "unknown",
                                "error": "Failed to read file"
                            }));
                            text_lines.push(format!("File {:?}: Failed to read file", path));
                            continue;
                        }
                    };

                    if let Ok(printer) = serde_json::from_str::<PrinterProfile>(&content) {
                        match printer.validate() {
                            Ok(()) => {
                                results.push(serde_json::json!({
                                    "file": path.to_string_lossy(),
                                    "valid": true,
                                    "type": "printer"
                                }));
                                text_lines.push(format!("Printer profile {:?} is valid.", path));
                            }
                            Err(err) => {
                                all_valid = false;
                                results.push(serde_json::json!({
                                    "file": path.to_string_lossy(),
                                    "valid": false,
                                    "type": "printer",
                                    "error": err
                                }));
                                text_lines.push(format!(
                                    "Printer profile {:?} is invalid: {}",
                                    path, err
                                ));
                            }
                        }
                    } else if let Ok(material) = serde_json::from_str::<MaterialProfile>(&content) {
                        match material.validate() {
                            Ok(()) => {
                                results.push(serde_json::json!({
                                    "file": path.to_string_lossy(),
                                    "valid": true,
                                    "type": "material"
                                }));
                                text_lines.push(format!("Material profile {:?} is valid.", path));
                            }
                            Err(err) => {
                                all_valid = false;
                                results.push(serde_json::json!({
                                    "file": path.to_string_lossy(),
                                    "valid": false,
                                    "type": "material",
                                    "error": err
                                }));
                                text_lines.push(format!(
                                    "Material profile {:?} is invalid: {}",
                                    path, err
                                ));
                            }
                        }
                    } else {
                        all_valid = false;
                        results.push(serde_json::json!({
                            "file": path.to_string_lossy(),
                            "valid": false,
                            "type": "unknown",
                            "error": "Failed to parse as printer or material profile"
                        }));
                        text_lines.push(format!(
                            "File {:?}: Failed to parse as printer or material profile",
                            path
                        ));
                    }
                }
            }

            let output_str = if is_json {
                serde_json::to_string_pretty(&results).unwrap() + "\n"
            } else {
                text_lines.join("\n") + "\n"
            };

            if let Some(out_path) = output {
                if let Err(e) = std::fs::write(&out_path, &output_str) {
                    eprintln!("Error: Failed to write report to {:?}: {}", out_path, e);
                    std::process::exit(1);
                }
            } else {
                print!("{}", output_str);
            }

            if !all_valid {
                std::process::exit(1);
            }
        }
    }
}

fn format_preflight_text(report: &printproof3d_core::ValidationReport) -> String {
    let mut s = String::new();
    use std::fmt::Write;
    writeln!(
        s,
        "============================================================"
    )
    .unwrap();
    writeln!(s, "PRINTPROOF3D PREFLIGHT VALIDATION SUMMARY").unwrap();
    writeln!(
        s,
        "============================================================"
    )
    .unwrap();
    writeln!(s, "Target Printer : {}", report.target_printer_profile).unwrap();
    writeln!(s, "Target Material: {}", report.target_material_profile).unwrap();
    writeln!(s, "Model/G-code   : {}", report.model.file_name).unwrap();
    writeln!(s, "Status         : {:?}", report.status).unwrap();
    writeln!(s, "Confidence     : {}", report.confidence_level).unwrap();
    writeln!(
        s,
        "------------------------------------------------------------"
    )
    .unwrap();

    let bbox = &report.model.bounding_box;
    writeln!(
        s,
        "Bounding Box   : X: [{:.2} to {:.2}], Y: [{:.2} to {:.2}], Z: [{:.2} to {:.2}] mm",
        bbox.min_x, bbox.max_x, bbox.min_y, bbox.max_y, bbox.min_z, bbox.max_z
    )
    .unwrap();

    if let Some(ref assumed) = report.sliced_settings_assumed {
        writeln!(s, "Assumed Slicer Settings:").unwrap();
        if let Some(obj) = assumed.as_object() {
            for (k, v) in obj {
                if k != "simulator_telemetry" {
                    writeln!(s, "  - {}: {}", k, v).unwrap();
                }
            }
            if let Some(telemetry) = obj.get("simulator_telemetry") {
                writeln!(s, "Simulator Telemetry:").unwrap();
                if let Some(t_obj) = telemetry.as_object() {
                    for (k, v) in t_obj {
                        writeln!(s, "  - {}: {}", k, v).unwrap();
                    }
                } else {
                    writeln!(s, "  - {:?}", telemetry).unwrap();
                }
            }
        } else {
            writeln!(s, "  - {:?}", assumed).unwrap();
        }
    }

    writeln!(
        s,
        "------------------------------------------------------------"
    )
    .unwrap();
    if report.issues.is_empty() {
        writeln!(
            s,
            "No issues detected by PrintProof3D profile and file validation checks."
        )
        .unwrap();
    } else {
        writeln!(s, "Issues detected ({}):", report.issues.len()).unwrap();
        for (i, issue) in report.issues.iter().enumerate() {
            writeln!(s, "\n  {}. [{:?}] ID: {}", i + 1, issue.severity, issue.id).unwrap();
            writeln!(s, "     Message: {}", issue.message).unwrap();
            if !issue.suggested_fixes.is_empty() {
                writeln!(s, "     Suggested Fixes:").unwrap();
                for fix in &issue.suggested_fixes {
                    writeln!(s, "       - {}", fix).unwrap();
                }
            }
        }
    }
    writeln!(
        s,
        "============================================================"
    )
    .unwrap();
    s
}

fn list_profiles_in_dir<T: serde::de::DeserializeOwned>(
    dir_path: &std::path::Path,
) -> Vec<(PathBuf, T)> {
    let mut profiles = Vec::new();
    let read_dir = match std::fs::read_dir(dir_path) {
        Ok(rd) => rd,
        Err(_) => return profiles,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(profile) = serde_json::from_str::<T>(&content) {
                    profiles.push((path, profile));
                }
            }
        }
    }
    profiles
}

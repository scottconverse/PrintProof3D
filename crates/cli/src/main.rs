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

        /// Path to write the output validation report JSON
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
    },
    /// Validate a printer profile JSON file against safety invariants
    ValidatePrinterProfile {
        /// Path to the printer profile JSON file
        path: PathBuf,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,
    },
    /// Validate a material profile JSON file against safety invariants
    ValidateMaterialProfile {
        /// Path to the material profile JSON file
        path: PathBuf,

        /// Output format (text, json)
        #[arg(long, short = 'f', default_value = "text")]
        format: String,
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
        Commands::InspectProfile { path, format } => {
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

            if let Ok(printer) = serde_json::from_str::<PrinterProfile>(&content) {
                if is_json {
                    let wrapped = serde_json::json!({
                        "profile_type": "printer",
                        "profile": printer,
                    });
                    println!("{}", serde_json::to_string_pretty(&wrapped).unwrap());
                } else {
                    println!("Profile Type    : Printer Profile");
                    println!("File Path       : {:?}", path);
                    println!("Manufacturer    : {}", printer.manufacturer);
                    println!("Model           : {}", printer.model);
                    println!("Protocol Family : {:?}", printer.protocol_family);
                    println!("Build Volume    : {:?}", printer.build_volume);
                    println!("Bed Shape       : {:?}", printer.bed_shape);
                    println!("Nozzle Diameters: {:?}", printer.nozzle_diameters);
                    println!(
                        "Default Nozzle  : {:.2} mm",
                        printer.default_nozzle_diameter
                    );
                    println!("Max Hotend Temp : {:.1}°C", printer.max_hotend_temp);
                    println!("Max Bed Temp    : {:.1}°C", printer.max_bed_temp);
                    println!("Has Enclosure   : {}", printer.has_enclosure);
                    println!("Supports MMU    : {}", printer.supports_mmu);
                    println!("Firmware Flavor : {:?}", printer.firmware_flavor);
                    println!("File Types      : {:?}", printer.supported_file_types);
                }
            } else if let Ok(material) = serde_json::from_str::<MaterialProfile>(&content) {
                if is_json {
                    let wrapped = serde_json::json!({
                        "profile_type": "material",
                        "profile": material,
                    });
                    println!("{}", serde_json::to_string_pretty(&wrapped).unwrap());
                } else {
                    println!("Profile Type    : Material Profile");
                    println!("File Path       : {:?}", path);
                    println!("Name            : {}", material.name);
                    println!("Abbreviations   : {:?}", material.abbreviations);
                    println!("Min Nozzle Temp : {:.1}°C", material.min_nozzle_temp);
                    println!("Max Nozzle Temp : {:.1}°C", material.max_nozzle_temp);
                    println!("Min Bed Temp    : {:.1}°C", material.min_bed_temp);
                    println!("Max Bed Temp    : {:.1}°C", material.max_bed_temp);
                    println!("Fan Speed Pct   : {:.1}%", material.cooling_fan_speed_pct);
                    println!("Warp Risk       : {:?}", material.warp_risk);
                    println!("Bridge Diff     : {:?}", material.bridge_difficulty);
                    println!("Overhang Diff   : {:?}", material.overhang_difficulty);
                    println!("Enclosure Rec   : {}", material.enclosure_recommended);
                    println!("Dryness Sens.   : {}", material.dryness_sensitive);
                    println!("Min Feature Size: {:.2} mm", material.min_feature_size_mm);
                }
            } else {
                eprintln!(
                    "Error: Structurally invalid profile or malformed JSON file at {:?}",
                    path
                );
                std::process::exit(1);
            }
        }
        Commands::ValidatePrinterProfile { path, format } => {
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
            match parsed {
                Ok(printer) => match printer.validate() {
                    Ok(()) => {
                        if is_json {
                            println!(
                                "{}",
                                serde_json::json!({ "valid": true, "type": "printer", "file": path.to_string_lossy() })
                            );
                        } else {
                            println!("Printer profile {:?} is valid.", path);
                        }
                    }
                    Err(e) => {
                        if is_json {
                            println!(
                                "{}",
                                serde_json::json!({ "valid": false, "type": "printer", "file": path.to_string_lossy(), "error": e })
                            );
                        } else {
                            eprintln!("Error: Printer profile validation failed: {}", e);
                        }
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({ "valid": false, "type": "printer", "file": path.to_string_lossy(), "error": e.to_string() })
                        );
                    } else {
                        eprintln!("Error: Failed to parse printer profile JSON: {}", e);
                    }
                    std::process::exit(1);
                }
            }
        }
        Commands::ValidateMaterialProfile { path, format } => {
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
            match parsed {
                Ok(material) => match material.validate() {
                    Ok(()) => {
                        if is_json {
                            println!(
                                "{}",
                                serde_json::json!({ "valid": true, "type": "material", "file": path.to_string_lossy() })
                            );
                        } else {
                            println!("Material profile {:?} is valid.", path);
                        }
                    }
                    Err(e) => {
                        if is_json {
                            println!(
                                "{}",
                                serde_json::json!({ "valid": false, "type": "material", "file": path.to_string_lossy(), "error": e })
                            );
                        } else {
                            eprintln!("Error: Material profile validation failed: {}", e);
                        }
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({ "valid": false, "type": "material", "file": path.to_string_lossy(), "error": e.to_string() })
                        );
                    } else {
                        eprintln!("Error: Failed to parse material profile JSON: {}", e);
                    }
                    std::process::exit(1);
                }
            }
        }
        Commands::CheckCompatibility {
            printer,
            material,
            model,
            gcode,
            format,
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
            if is_json {
                let report = serde_json::json!({
                    "status": format!("{:?}", status).to_lowercase(),
                    "printer": format!("{}_{}", printer_profile.manufacturer, printer_profile.model),
                    "material": resolved_material.map(|m| m.name),
                    "model": model.map(|p| p.file_name().unwrap().to_string_lossy().into_owned()),
                    "gcode": gcode.map(|p| p.file_name().unwrap().to_string_lossy().into_owned()),
                    "issues": issues,
                });
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                println!("============================================================");
                println!("PRINTPROOF3D COMPATIBILITY REPORT");
                println!("============================================================");
                println!(
                    "Printer  : {} {}",
                    printer_profile.manufacturer, printer_profile.model
                );
                if let Some(ref m) = resolved_material {
                    println!("Material : {}", m.name);
                }
                if let Some(ref m_path) = model {
                    println!("Model    : {:?}", m_path.file_name().unwrap());
                }
                if let Some(ref g_path) = gcode {
                    println!("G-code   : {:?}", g_path.file_name().unwrap());
                }
                println!("Status   : {:?}", status);
                println!("------------------------------------------------------------");
                if issues.is_empty() {
                    println!("No compatibility issues detected.");
                    println!("Target passes PrintProof3D profile and file validation checks.");
                } else {
                    println!("Issues detected ({}):", issues.len());
                    for (i, issue) in issues.iter().enumerate() {
                        println!("\n  {}. [{:?}] ID: {}", i + 1, issue.severity, issue.id);
                        println!("     Message: {}", issue.message);
                        if !issue.suggested_fixes.is_empty() {
                            println!("     Suggested Fixes:");
                            for fix in &issue.suggested_fixes {
                                println!("       - {}", fix);
                            }
                        }
                    }
                }
                println!("============================================================");
            }

            if status == ValidationStatus::Fail {
                std::process::exit(1);
            }
        }
    }
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

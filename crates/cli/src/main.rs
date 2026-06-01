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
    }
}

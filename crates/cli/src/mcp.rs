use printproof3d_core::{MaterialProfile, PrinterProfile, ValidationReport, ValidationStatus};
use printproof3d_printability::{
    GcodeValidator, ModelValidator, StandardGcodeValidator, StlModelValidator,
};
use serde::{Deserialize, Serialize};
use std::io::{stdin, stdout, BufRead, Write};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Option<serde_json::Value>,
}

pub fn run_mcp_server() {
    let stdin = stdin();
    let stdout = stdout();
    run_mcp_loop(stdin.lock(), stdout.lock());
}

pub fn run_mcp_loop<R: BufRead, W: Write>(mut reader: R, mut writer: W) {
    let mut line = String::new();

    while let Ok(bytes) = reader.read_line(&mut line) {
        if bytes == 0 {
            break;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
                let resp = handle_mcp_request(req);
                if let Ok(resp_json) = serde_json::to_string(&resp) {
                    let _ = writeln!(writer, "{}", resp_json);
                    let _ = writer.flush();
                }
            }
        }
        line.clear();
    }
}

fn handle_mcp_request(req: JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "printproof3d-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }
        }
        "tools/list" => {
            let result = serde_json::json!({
                "tools": [
                    {
                        "name": "validate_model_printability",
                        "description": "Validate a 3D model mesh (STL file) against a printer profile and material profile.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "model_path": { "type": "string", "description": "Absolute path to the STL mesh file." },
                                "printer_profile_path": { "type": "string", "description": "Absolute path to the printer profile JSON file." },
                                "material_profile_path": { "type": "string", "description": "Absolute path to the material profile JSON file." }
                            },
                            "required": ["model_path", "printer_profile_path", "material_profile_path"]
                        }
                    },
                    {
                        "name": "validate_gcode",
                        "description": "Validate G-code file constraints (dimensions and temperatures) against printer profile and material profile.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "gcode_path": { "type": "string", "description": "Absolute path to the G-code file." },
                                "printer_profile_path": { "type": "string", "description": "Absolute path to the printer profile JSON file." },
                                "material_profile_path": { "type": "string", "description": "Optional absolute path to the material profile JSON file." }
                            },
                            "required": ["gcode_path", "printer_profile_path"]
                        }
                    },
                    {
                        "name": "list_printer_profiles",
                        "description": "List all default printer profiles configured in the workspace.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "explain_validation_report",
                        "description": "Provide a detailed, plain-language explanation of a validation report JSON structure and its issues.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "report_json": { "type": "string", "description": "The validation report JSON string." }
                            },
                            "required": ["report_json"]
                        }
                    }
                ]
            });
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }
        }
        "tools/call" => {
            if let Some(params) = req.params {
                if let Ok(call) = serde_json::from_value::<ToolCallParams>(params) {
                    return match execute_tool(call) {
                        Ok(res) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(res),
                            error: None,
                        },
                        Err(err_msg) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32000,
                                message: err_msg,
                                data: None,
                            }),
                        },
                    };
                }
            }
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid parameters".to_string(),
                    data: None,
                }),
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
                data: None,
            }),
        },
    }
}

fn execute_tool(call: ToolCallParams) -> Result<serde_json::Value, String> {
    let args = call.arguments.unwrap_or(serde_json::Value::Null);

    match call.name.as_str() {
        "validate_model_printability" => {
            let model_path = args
                .get("model_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'model_path'")?;
            let printer_path = args
                .get("printer_profile_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'printer_profile_path'")?;
            let material_path = args
                .get("material_profile_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'material_profile_path'")?;

            let printer_json = std::fs::read_to_string(printer_path)
                .map_err(|e| format!("Failed to read printer profile: {}", e))?;
            let printer: PrinterProfile = serde_json::from_str(&printer_json)
                .map_err(|e| format!("Malformed printer profile: {}", e))?;
            printer
                .validate()
                .map_err(|e| format!("Invalid printer profile: {}", e))?;

            let material_json = std::fs::read_to_string(material_path)
                .map_err(|e| format!("Failed to read material profile: {}", e))?;
            let material: MaterialProfile = serde_json::from_str(&material_json)
                .map_err(|e| format!("Malformed material profile: {}", e))?;
            material
                .validate()
                .map_err(|e| format!("Invalid material profile: {}", e))?;

            let validator = StlModelValidator;
            let report = validator.validate_mesh(Path::new(model_path), &printer, &material)?;
            let report_json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;

            Ok(serde_json::json!({
                "content": [
                    { "type": "text", "text": report_json }
                ]
            }))
        }
        "validate_gcode" => {
            let gcode_path = args
                .get("gcode_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'gcode_path'")?;
            let printer_path = args
                .get("printer_profile_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'printer_profile_path'")?;
            let material_path = args.get("material_profile_path").and_then(|v| v.as_str());

            let printer_json = std::fs::read_to_string(printer_path)
                .map_err(|e| format!("Failed to read printer profile: {}", e))?;
            let printer: PrinterProfile = serde_json::from_str(&printer_json)
                .map_err(|e| format!("Malformed printer profile: {}", e))?;
            printer
                .validate()
                .map_err(|e| format!("Invalid printer profile: {}", e))?;

            let material = if let Some(m_path) = material_path {
                let material_json = std::fs::read_to_string(m_path)
                    .map_err(|e| format!("Failed to read material profile: {}", e))?;
                let material: MaterialProfile = serde_json::from_str(&material_json)
                    .map_err(|e| format!("Malformed material profile: {}", e))?;
                material
                    .validate()
                    .map_err(|e| format!("Invalid material profile: {}", e))?;
                material
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

            let validator = StandardGcodeValidator;
            let report = validator.validate_gcode(Path::new(gcode_path), &printer, &material)?;
            let report_json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;

            Ok(serde_json::json!({
                "content": [
                    { "type": "text", "text": report_json }
                ]
            }))
        }
        "list_printer_profiles" => {
            let mut profiles_dir = std::env::current_dir().unwrap_or_default().join("profiles");
            if !profiles_dir.exists() {
                profiles_dir = std::env::current_dir()
                    .unwrap_or_default()
                    .join("../../profiles");
            }
            if !profiles_dir.exists() {
                return Err("Profiles directory not found".to_string());
            }

            let mut profiles = Vec::new();
            let entries = std::fs::read_dir(profiles_dir).map_err(|e| e.to_string())?;
            for entry_opt in entries {
                let entry = entry_opt.map_err(|e| e.to_string())?;
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let contents = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                    if let Ok(p) = serde_json::from_str::<PrinterProfile>(&contents) {
                        if p.validate().is_ok() {
                            profiles.push(p);
                        }
                    }
                }
            }

            let profiles_json =
                serde_json::to_string_pretty(&profiles).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "content": [
                    { "type": "text", "text": profiles_json }
                ]
            }))
        }
        "explain_validation_report" => {
            let report_str = args
                .get("report_json")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'report_json'")?;
            let report: ValidationReport = serde_json::from_str(report_str)
                .map_err(|e| format!("Malformed validation report: {}", e))?;

            let mut explanation = format!(
                "PrintProof3D Validation Report Summary:\n- File: {}\n- Printer Target: {}\n- Material Target: {}\n- Status: {}\n",
                report.model.file_name,
                report.target_printer_profile,
                report.target_material_profile,
                match report.status {
                    ValidationStatus::Pass => "PASS (Ready to print)",
                    ValidationStatus::Warning => "WARNING (Review warnings before printing)",
                    ValidationStatus::Fail => "FAIL (Printing blocked / dangerous safety issues found)",
                }
            );

            if report.issues.is_empty() {
                explanation.push_str(
                    "\nNo issues were found. The print matches all safety invariants and bounds.",
                );
            } else {
                explanation.push_str(&format!(
                    "\nFound {} validation alerts:\n",
                    report.issues.len()
                ));
                for (idx, issue) in report.issues.iter().enumerate() {
                    explanation.push_str(&format!(
                        "\n[{}] {} ({:?}): {}\n   Suggested fixes:\n",
                        idx + 1,
                        issue.id,
                        issue.severity,
                        issue.message
                    ));
                    for fix in &issue.suggested_fixes {
                        explanation.push_str(&format!("   - {}\n", fix));
                    }
                }
            }

            Ok(serde_json::json!({
                "content": [
                    { "type": "text", "text": explanation }
                ]
            }))
        }
        _ => Err(format!("Unknown tool: {}", call.name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: None,
        };
        let resp = handle_mcp_request(req);
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, serde_json::json!(1));
        let res = resp.result.unwrap();
        assert_eq!(res.get("protocolVersion").unwrap(), "2024-11-05");
        assert!(res.get("serverInfo").is_some());
    }

    #[test]
    fn test_mcp_tools_list() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };
        let resp = handle_mcp_request(req);
        assert_eq!(resp.jsonrpc, "2.0");
        let res = resp.result.unwrap();
        let tools = res.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 4);
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap() == "validate_model_printability"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap() == "validate_gcode"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap() == "list_printer_profiles"));
        assert!(tools
            .iter()
            .any(|t| t.get("name").unwrap() == "explain_validation_report"));
    }

    #[test]
    fn test_mcp_explain_validation_report() {
        let report_json = serde_json::json!({
            "status": "pass",
            "target_printer_profile": "Prusa_MK4",
            "target_material_profile": "PLA",
            "model": {
                "file_name": "test.stl",
                "units": "mm",
                "bounding_box": {
                    "min_x": 0.0,
                    "min_y": 0.0,
                    "min_z": 0.0,
                    "max_x": 10.0,
                    "max_y": 10.0,
                    "max_z": 10.0
                }
            },
            "issues": [],
            "confidence_level": "high",
            "sliced_settings_assumed": null
        })
        .to_string();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "explain_validation_report",
                "arguments": {
                    "report_json": report_json
                }
            })),
        };
        let resp = handle_mcp_request(req);
        assert!(resp.error.is_none());
        let res = resp.result.unwrap();
        let content = res.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        assert!(text.contains("PASS"));
        assert!(text.contains("No issues were found"));
    }

    #[test]
    fn test_mcp_validate_model_printability() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let model_path = manifest_dir
            .join("../../fixtures/tetrahedron.stl")
            .to_string_lossy()
            .into_owned();
        let printer_path = manifest_dir
            .join("../../profiles/prusa_mk4.json")
            .to_string_lossy()
            .into_owned();
        let material_path = manifest_dir
            .join("../../profiles/pla.json")
            .to_string_lossy()
            .into_owned();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(4)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "validate_model_printability",
                "arguments": {
                    "model_path": model_path,
                    "printer_profile_path": printer_path,
                    "material_profile_path": material_path
                }
            })),
        };
        let resp = handle_mcp_request(req);
        assert!(resp.error.is_none());
        let res = resp.result.unwrap();
        let content = res.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        assert!(text.contains("Prusa_MK4"));
        assert!(text.contains("Polylactic Acid"));
    }

    #[test]
    fn test_mcp_validate_gcode() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let gcode_path = manifest_dir
            .join("../../fixtures/safe_print.gcode")
            .to_string_lossy()
            .into_owned();
        let printer_path = manifest_dir
            .join("../../profiles/prusa_mk4.json")
            .to_string_lossy()
            .into_owned();
        let material_path = manifest_dir
            .join("../../profiles/pla.json")
            .to_string_lossy()
            .into_owned();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(5)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "validate_gcode",
                "arguments": {
                    "gcode_path": gcode_path,
                    "printer_profile_path": printer_path,
                    "material_profile_path": material_path
                }
            })),
        };
        let resp = handle_mcp_request(req);
        assert!(resp.error.is_none());
        let res = resp.result.unwrap();
        let content = res.get("content").unwrap().as_array().unwrap();
        let text = content[0].get("text").unwrap().as_str().unwrap();
        assert!(text.contains("Prusa_MK4"));
        assert!(text.contains("Polylactic Acid"));
    }

    #[test]
    fn test_run_mcp_loop() {
        let input_lines = r#"
{"jsonrpc":"2.0","id":10,"method":"initialize"}
{"jsonrpc":"2.0","id":11,"method":"tools/list"}
"#;
        let reader = std::io::Cursor::new(input_lines);
        let mut writer = Vec::new();
        run_mcp_loop(reader, &mut writer);

        let output = String::from_utf8(writer).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);

        let resp1: JsonRpcResponse = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(resp1.id, serde_json::json!(10));
        assert!(resp1.result.is_some());

        let resp2: JsonRpcResponse = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(resp2.id, serde_json::json!(11));
        assert!(resp2.result.is_some());
    }
}

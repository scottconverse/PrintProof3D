// Trigger rebuild of embedded index.html
use axum::{
    extract::Multipart,
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use printproof3d_core::{MaterialProfile, PrinterProfile, ValidationReport, ValidationStatus};
use printproof3d_printability::{
    GcodeValidator, ModelValidator, StandardGcodeValidator, StlModelValidator,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use tower_http::cors::{AllowOrigin, CorsLayer};

const INDEX_HTML: &str = include_str!("../../../index.html");
const USER_MANUAL_HTML: &str = include_str!("../../../user_manual.html");
const API_REFERENCE_HTML: &str = include_str!("../../../api_reference.html");
const ARCHITECTURE_HTML: &str = include_str!("../../../architecture.html");

const THREE_JS: &str = include_str!("../assets/three.min.js");
const STL_LOADER: &str = include_str!("../assets/STLLoader.js");
const ORBIT_CONTROLS: &str = include_str!("../assets/OrbitControls.js");

const BAMBU_X1C_JSON: &str = include_str!("../../../profiles/bambu_x1c.json");
const DUET_RRF_JSON: &str = include_str!("../../../profiles/duet_rrf.json");
const ENDER3_SERIAL_JSON: &str = include_str!("../../../profiles/ender3_serial.json");
const GENERIC_OCTOPRINT_JSON: &str = include_str!("../../../profiles/generic_octoprint.json");
const PRUSA_MK4_JSON: &str = include_str!("../../../profiles/prusa_mk4.json");
const VORON_KLIPPER_JSON: &str = include_str!("../../../profiles/voron_klipper.json");
const PLA_JSON: &str = include_str!("../../../profiles/pla.json");

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);
static EPHEMERAL_TOKEN: OnceLock<String> = OnceLock::new();

fn generate_random_token() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    let val1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    val1.hash(&mut hasher2);
    let val2 = hasher2.finish();

    format!("{:016x}{:016x}", val1, val2)
}

fn get_api_token() -> &'static str {
    if let Ok(token) = std::env::var("PRINTPROOF3D_API_TOKEN") {
        return Box::leak(token.into_boxed_str());
    }

    if cfg!(test) {
        return "secret_print_token";
    }

    EPHEMERAL_TOKEN.get_or_init(|| {
        let token = generate_random_token();
        println!(
            "[PrintProof3D API] Token is not configured. Ephemeral token generated: {}",
            token
        );
        token
    })
}

fn unique_temp_file_name(original_name: &str) -> std::path::PathBuf {
    let pid = std::process::id();
    let count = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("temp_uploads");
    temp_dir.join(format!("{}_{}_{}", pid, count, original_name))
}

async fn auth_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok());

    let target_token = get_api_token();
    let expected_auth = format!("Bearer {}", target_token);

    if let Some(auth) = auth_header {
        if auth == expected_auth {
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

async fn serve_dashboard() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn serve_manual() -> impl IntoResponse {
    Html(USER_MANUAL_HTML)
}

async fn serve_api() -> impl IntoResponse {
    Html(API_REFERENCE_HTML)
}

async fn serve_architecture() -> impl IntoResponse {
    Html(ARCHITECTURE_HTML)
}

async fn serve_three() -> Response {
    Response::builder()
        .header("content-type", "application/javascript")
        .body(axum::body::Body::from(THREE_JS))
        .unwrap()
}

async fn serve_loader() -> Response {
    Response::builder()
        .header("content-type", "application/javascript")
        .body(axum::body::Body::from(STL_LOADER))
        .unwrap()
}

async fn serve_controls() -> Response {
    Response::builder()
        .header("content-type", "application/javascript")
        .body(axum::body::Body::from(ORBIT_CONTROLS))
        .unwrap()
}

async fn list_printer_profiles() -> Result<axum::Json<Vec<PrinterProfile>>, (StatusCode, String)> {
    let raw_profiles = vec![
        BAMBU_X1C_JSON,
        DUET_RRF_JSON,
        ENDER3_SERIAL_JSON,
        GENERIC_OCTOPRINT_JSON,
        PRUSA_MK4_JSON,
        VORON_KLIPPER_JSON,
    ];
    let mut profiles = Vec::new();
    for contents in raw_profiles {
        if let Ok(p) = serde_json::from_str::<PrinterProfile>(contents) {
            if p.validate().is_ok() {
                profiles.push(p);
            }
        }
    }
    Ok(axum::Json(profiles))
}

async fn validate_model(
    mut multipart: Multipart,
) -> Result<axum::Json<ValidationReport>, (StatusCode, String)> {
    let mut model_bytes = None;
    let mut model_name = "model.stl".to_string();
    let mut printer_profile = None;
    let mut material_profile = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "model" {
            model_name = field.file_name().unwrap_or("model.stl").to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            model_bytes = Some(data.to_vec());
        } else if name == "printer" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            let p: PrinterProfile = serde_json::from_slice(&data).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Malformed printer profile: {}", e),
                )
            })?;
            p.validate().map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid printer profile: {}", e),
                )
            })?;
            printer_profile = Some(p);
        } else if name == "material" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            let m: MaterialProfile = serde_json::from_slice(&data).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Malformed material profile: {}", e),
                )
            })?;
            m.validate().map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid material profile: {}", e),
                )
            })?;
            material_profile = Some(m);
        }
    }

    let model_bytes =
        model_bytes.ok_or((StatusCode::BAD_REQUEST, "Missing 'model' file".to_string()))?;
    let printer = printer_profile.ok_or((
        StatusCode::BAD_REQUEST,
        "Missing 'printer' profile".to_string(),
    ))?;
    let material = material_profile.ok_or((
        StatusCode::BAD_REQUEST,
        "Missing 'material' profile".to_string(),
    ))?;

    let temp_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("temp_uploads");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let temp_file_path = unique_temp_file_name(&model_name);
    std::fs::write(&temp_file_path, &model_bytes)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let validator = StlModelValidator;
    let report = validator
        .validate_mesh(&temp_file_path, &printer, &material)
        .map_err(|e| {
            let _ = std::fs::remove_file(&temp_file_path);
            (StatusCode::INTERNAL_SERVER_ERROR, e)
        })?;

    let _ = std::fs::remove_file(&temp_file_path);
    Ok(axum::Json(report))
}

async fn validate_gcode(
    mut multipart: Multipart,
) -> Result<axum::Json<ValidationReport>, (StatusCode, String)> {
    let mut gcode_bytes = None;
    let mut gcode_name = "print.gcode".to_string();
    let mut printer_profile = None;
    let mut material_profile = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "gcode" {
            gcode_name = field.file_name().unwrap_or("print.gcode").to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            gcode_bytes = Some(data.to_vec());
        } else if name == "printer" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            let p: PrinterProfile = serde_json::from_slice(&data).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Malformed printer profile: {}", e),
                )
            })?;
            p.validate().map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid printer profile: {}", e),
                )
            })?;
            printer_profile = Some(p);
        } else if name == "material" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            let m: MaterialProfile = serde_json::from_slice(&data).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Malformed material profile: {}", e),
                )
            })?;
            m.validate().map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid material profile: {}", e),
                )
            })?;
            material_profile = Some(m);
        }
    }

    let gcode_bytes =
        gcode_bytes.ok_or((StatusCode::BAD_REQUEST, "Missing 'gcode' file".to_string()))?;
    let printer = printer_profile.ok_or((
        StatusCode::BAD_REQUEST,
        "Missing 'printer' profile".to_string(),
    ))?;

    let material = material_profile.unwrap_or_else(|| MaterialProfile {
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

    let temp_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("temp_uploads");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let temp_file_path = unique_temp_file_name(&gcode_name);
    std::fs::write(&temp_file_path, &gcode_bytes)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let validator = StandardGcodeValidator;
    let report = validator
        .validate_gcode(&temp_file_path, &printer, &material)
        .map_err(|e| {
            let _ = std::fs::remove_file(&temp_file_path);
            (StatusCode::INTERNAL_SERVER_ERROR, e)
        })?;

    let _ = std::fs::remove_file(&temp_file_path);
    Ok(axum::Json(report))
}

async fn list_material_profiles() -> Result<axum::Json<Vec<MaterialProfile>>, (StatusCode, String)>
{
    let raw_profiles = vec![PLA_JSON];
    let mut profiles = Vec::new();
    for contents in raw_profiles {
        if let Ok(m) = serde_json::from_str::<MaterialProfile>(contents) {
            if m.validate().is_ok() {
                profiles.push(m);
            }
        }
    }
    Ok(axum::Json(profiles))
}

async fn inspect_profile(
    mut multipart: Multipart,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    let mut profile_bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "profile" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            profile_bytes = Some(data.to_vec());
        }
    }

    let bytes = profile_bytes.ok_or((
        StatusCode::BAD_REQUEST,
        "Missing 'profile' file".to_string(),
    ))?;

    if let Ok(printer) = serde_json::from_slice::<PrinterProfile>(&bytes) {
        if printer.validate().is_ok() {
            return Ok(axum::Json(serde_json::json!({
                "profile_type": "printer",
                "profile": printer
            })));
        }
    }

    if let Ok(material) = serde_json::from_slice::<MaterialProfile>(&bytes) {
        if material.validate().is_ok() {
            return Ok(axum::Json(serde_json::json!({
                "profile_type": "material",
                "profile": material
            })));
        }
    }

    Err((
        StatusCode::BAD_REQUEST,
        "Invalid profile JSON format".to_string(),
    ))
}

async fn validate_printer_profile_route(
    mut multipart: Multipart,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, axum::Json<serde_json::Value>)> {
    let mut profile_bytes = None;
    let mut file_name = "uploaded_profile.json".to_string();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "valid": false,
                "type": "printer",
                "file": "uploaded_profile.json",
                "error": e.to_string()
            })),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == "printer" {
            file_name = field
                .file_name()
                .unwrap_or("uploaded_profile.json")
                .to_string();
            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({
                        "valid": false,
                        "type": "printer",
                        "file": file_name.clone(),
                        "error": e.to_string()
                    })),
                )
            })?;
            profile_bytes = Some(data.to_vec());
        }
    }

    let bytes = match profile_bytes {
        Some(b) => b,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "valid": false,
                    "type": "printer",
                    "file": file_name,
                    "error": "Missing 'printer' file"
                })),
            ))
        }
    };

    let printer = match serde_json::from_slice::<PrinterProfile>(&bytes) {
        Ok(p) => p,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "valid": false,
                    "type": "printer",
                    "file": file_name,
                    "error": format!("Malformed printer profile: {}", e)
                })),
            ))
        }
    };

    match printer.validate() {
        Ok(()) => Ok(axum::Json(serde_json::json!({
            "valid": true,
            "type": "printer",
            "file": file_name
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "valid": false,
                "type": "printer",
                "file": file_name,
                "error": format!("Invalid printer profile: {}", e)
            })),
        )),
    }
}

async fn validate_material_profile_route(
    mut multipart: Multipart,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, axum::Json<serde_json::Value>)> {
    let mut profile_bytes = None;
    let mut file_name = "uploaded_profile.json".to_string();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "valid": false,
                "type": "material",
                "file": "uploaded_profile.json",
                "error": e.to_string()
            })),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == "material" {
            file_name = field
                .file_name()
                .unwrap_or("uploaded_profile.json")
                .to_string();
            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({
                        "valid": false,
                        "type": "material",
                        "file": file_name.clone(),
                        "error": e.to_string()
                    })),
                )
            })?;
            profile_bytes = Some(data.to_vec());
        }
    }

    let bytes = match profile_bytes {
        Some(b) => b,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "valid": false,
                    "type": "material",
                    "file": file_name,
                    "error": "Missing 'material' file"
                })),
            ))
        }
    };

    let material = match serde_json::from_slice::<MaterialProfile>(&bytes) {
        Ok(m) => m,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "valid": false,
                    "type": "material",
                    "file": file_name,
                    "error": format!("Malformed material profile: {}", e)
                })),
            ))
        }
    };

    match material.validate() {
        Ok(()) => Ok(axum::Json(serde_json::json!({
            "valid": true,
            "type": "material",
            "file": file_name
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "valid": false,
                "type": "material",
                "file": file_name,
                "error": format!("Invalid material profile: {}", e)
            })),
        )),
    }
}

async fn validate_compatibility(
    mut multipart: Multipart,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    let mut printer_profile = None;
    let mut material_profile = None;
    let mut model_bytes = None;
    let mut model_name = None;
    let mut gcode_bytes = None;
    let mut gcode_name = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "printer" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            let p: PrinterProfile = serde_json::from_slice(&data).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Malformed printer profile: {}", e),
                )
            })?;
            p.validate().map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid printer profile: {}", e),
                )
            })?;
            printer_profile = Some(p);
        } else if name == "material" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            let m: MaterialProfile = serde_json::from_slice(&data).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Malformed material profile: {}", e),
                )
            })?;
            m.validate().map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid material profile: {}", e),
                )
            })?;
            material_profile = Some(m);
        } else if name == "model" {
            model_name = Some(field.file_name().unwrap_or("model.stl").to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            model_bytes = Some(data.to_vec());
        } else if name == "gcode" {
            gcode_name = Some(field.file_name().unwrap_or("print.gcode").to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            gcode_bytes = Some(data.to_vec());
        }
    }

    let printer = printer_profile.ok_or((
        StatusCode::BAD_REQUEST,
        "Missing 'printer' profile".to_string(),
    ))?;

    let mut issues = Vec::new();

    if let Some(ref mat) = material_profile {
        let mat_issues =
            printproof3d_printability::compatibility::check_printer_material_compatibility(
                &printer, mat,
            );
        issues.extend(mat_issues);
    }

    let mat_ref = material_profile.clone().unwrap_or_else(|| MaterialProfile {
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

    let temp_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("temp_uploads");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(ref m_bytes) = model_bytes {
        let m_name = model_name.as_ref().unwrap();
        let temp_file_path = unique_temp_file_name(m_name);
        std::fs::write(&temp_file_path, m_bytes)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let validator = StlModelValidator;
        let model_report_res = validator.validate_mesh(&temp_file_path, &printer, &mat_ref);
        let _ = std::fs::remove_file(&temp_file_path);

        match model_report_res {
            Ok(report) => {
                issues.extend(report.issues);
            }
            Err(err) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, err));
            }
        }
    } else if let Some(ref g_bytes) = gcode_bytes {
        let g_name = gcode_name.as_ref().unwrap();
        let temp_file_path = unique_temp_file_name(g_name);
        std::fs::write(&temp_file_path, g_bytes)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let validator = StandardGcodeValidator;
        let gcode_report_res = validator.validate_gcode(&temp_file_path, &printer, &mat_ref);
        let _ = std::fs::remove_file(&temp_file_path);

        match gcode_report_res {
            Ok(report) => {
                issues.extend(report.issues);
            }
            Err(err) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, err));
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

    Ok(axum::Json(serde_json::json!({
        "status": format!("{:?}", status).to_lowercase(),
        "printer": format!("{}_{}", printer.manufacturer, printer.model),
        "material": material_profile.map(|m| m.name),
        "model": model_name,
        "gcode": gcode_name,
        "issues": issues,
    })))
}

pub fn api_router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(
            |origin: &HeaderValue, _request_parts| {
                if let Ok(origin_str) = origin.to_str() {
                    origin_str == "http://localhost"
                        || origin_str.starts_with("http://localhost:")
                        || origin_str == "http://127.0.0.1"
                        || origin_str.starts_with("http://127.0.0.1:")
                } else {
                    false
                }
            },
        ))
        .allow_headers(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any);

    Router::new()
        .route("/", get(serve_dashboard))
        .route("/docs/user_manual", get(serve_manual))
        .route("/docs/api_reference", get(serve_api))
        .route("/docs/architecture", get(serve_architecture))
        .route("/assets/three.min.js", get(serve_three))
        .route("/assets/STLLoader.js", get(serve_loader))
        .route("/assets/OrbitControls.js", get(serve_controls))
        .route("/profiles/printers", get(list_printer_profiles))
        .route("/profiles/materials", get(list_material_profiles))
        .route(
            "/validate/model",
            axum::routing::post(validate_model).route_layer(middleware::from_fn(auth_middleware)),
        )
        .route(
            "/validate/gcode",
            axum::routing::post(validate_gcode).route_layer(middleware::from_fn(auth_middleware)),
        )
        .route(
            "/profiles/inspect",
            axum::routing::post(inspect_profile).route_layer(middleware::from_fn(auth_middleware)),
        )
        .route(
            "/profiles/validate/printer",
            axum::routing::post(validate_printer_profile_route)
                .route_layer(middleware::from_fn(auth_middleware)),
        )
        .route(
            "/profiles/validate/material",
            axum::routing::post(validate_material_profile_route)
                .route_layer(middleware::from_fn(auth_middleware)),
        )
        .route(
            "/validate/compatibility",
            axum::routing::post(validate_compatibility)
                .route_layer(middleware::from_fn(auth_middleware)),
        )
        .layer(cors)
}

#[tokio::main]
async fn main() {
    // Force token generation/printing on startup
    let _ = get_api_token();
    let app = api_router();

    let port: u16 = std::env::var("PRINTPROOF3D_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_printer_profiles() {
        let res = list_printer_profiles().await;
        assert!(res.is_ok());
        let profiles = res.unwrap().0;
        assert!(!profiles.is_empty());
        assert!(profiles.iter().any(|p| p.model == "MK4"));
    }

    #[tokio::test]
    async fn test_home_route() {
        use axum::http::Request;
        use tower::ServiceExt;

        let app = api_router();
        let req = Request::builder()
            .method("GET")
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let content_type = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(content_type.contains("text/html"));
    }

    #[tokio::test]
    async fn test_cors_origin_validation() {
        use axum::http::{header, Request};
        use tower::ServiceExt;

        let app = api_router();

        // 1. Valid origin http://localhost:3000
        let req = Request::builder()
            .method("GET")
            .uri("/")
            .header(header::ORIGIN, "http://localhost:3000")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "http://localhost:3000"
        );

        // 2. Invalid origin http://malicious.com
        let req = Request::builder()
            .method("GET")
            .uri("/")
            .header(header::ORIGIN, "http://malicious.com")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none());
    }

    #[tokio::test]
    async fn test_auth_middleware_integration() {
        use axum::http::{header, Request};
        use tower::ServiceExt;

        let app = api_router();

        // 1. Missing Authorization header -> 401
        let req = Request::builder()
            .method("POST")
            .uri("/validate/model")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // 2. Incorrect token -> 401
        let req = Request::builder()
            .method("POST")
            .uri("/validate/model")
            .header(header::AUTHORIZATION, "Bearer wrong_token")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    fn create_multipart_body(
        boundary: &str,
        fields: &[(&str, &str, Option<&str>, &[u8])],
    ) -> axum::body::Body {
        let mut body = Vec::new();
        for &(name, filename, mime, data) in fields {
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            if !filename.is_empty() {
                let mime_str = mime.unwrap_or("application/octet-stream");
                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                        name, filename, mime_str
                    )
                    .as_bytes(),
                );
            } else {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", name).as_bytes(),
                );
            }
            body.extend_from_slice(data);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
        axum::body::Body::from(body)
    }

    #[tokio::test]
    async fn test_new_routes_auth() {
        use axum::http::Request;
        use tower::ServiceExt;

        let app = api_router();

        // GET /profiles/materials does not require auth
        let req = Request::builder()
            .method("GET")
            .uri("/profiles/materials")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // POST /profiles/inspect requires auth
        let req = Request::builder()
            .method("POST")
            .uri("/profiles/inspect")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // POST /profiles/validate/printer requires auth
        let req = Request::builder()
            .method("POST")
            .uri("/profiles/validate/printer")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // POST /profiles/validate/material requires auth
        let req = Request::builder()
            .method("POST")
            .uri("/profiles/validate/material")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // POST /validate/compatibility requires auth
        let req = Request::builder()
            .method("POST")
            .uri("/validate/compatibility")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_profiles_validation_and_compatibility() {
        use axum::http::{header, Request};
        use tower::ServiceExt;

        let app = api_router();
        let boundary = "---------------------------1234567890";
        let token = "Bearer secret_print_token";

        // 1. Test validate printer profile
        let printer_json = r#"{
            "manufacturer": "Prusa",
            "model": "MK4",
            "protocol_family": "prusa_link",
            "build_volume": { "type": "rectangular", "x": 250, "y": 210, "z": 220 },
            "bed_shape": "rectangular",
            "nozzle_diameters": [0.4],
            "default_nozzle_diameter": 0.4,
            "min_layer_height": 0.05,
            "max_layer_height": 0.3,
            "max_hotend_temp": 300,
            "max_bed_temp": 120,
            "has_enclosure": false,
            "supports_mmu": true,
            "firmware_flavor": "prusa",
            "supported_file_types": ["gcode"],
            "supports_direct_upload": true,
            "supports_pause_resume": true,
            "supports_cancel": true,
            "supports_job_progress": true,
            "supports_webcam": false,
            "supports_chamber_temp": false,
            "known_quirks": [],
            "unsafe_commands": ["M500"]
        }"#;

        let body = create_multipart_body(
            boundary,
            &[(
                "printer",
                "prusa_mk4.json",
                Some("application/json"),
                printer_json.as_bytes(),
            )],
        );
        let req = Request::builder()
            .method("POST")
            .uri("/profiles/validate/printer")
            .header(header::AUTHORIZATION, token)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 2. Test validate material profile
        let material_json = r#"{
            "name": "Polylactic Acid",
            "abbreviations": ["PLA"],
            "min_nozzle_temp": 190.0,
            "max_nozzle_temp": 220.0,
            "min_bed_temp": 50.0,
            "max_bed_temp": 60.0,
            "cooling_fan_speed_pct": 100.0,
            "warp_risk": "low",
            "bridge_difficulty": "low",
            "overhang_difficulty": "low",
            "enclosure_recommended": false,
            "dryness_sensitive": false,
            "min_feature_size_mm": 0.4
        }"#;

        let body = create_multipart_body(
            boundary,
            &[(
                "material",
                "pla.json",
                Some("application/json"),
                material_json.as_bytes(),
            )],
        );
        let req = Request::builder()
            .method("POST")
            .uri("/profiles/validate/material")
            .header(header::AUTHORIZATION, token)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 3. Test inspect profile (printer)
        let body = create_multipart_body(
            boundary,
            &[(
                "profile",
                "prusa_mk4.json",
                Some("application/json"),
                printer_json.as_bytes(),
            )],
        );
        let req = Request::builder()
            .method("POST")
            .uri("/profiles/inspect")
            .header(header::AUTHORIZATION, token)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 4. Test validate compatibility (printer + material)
        let body = create_multipart_body(
            boundary,
            &[
                (
                    "printer",
                    "prusa_mk4.json",
                    Some("application/json"),
                    printer_json.as_bytes(),
                ),
                (
                    "material",
                    "pla.json",
                    Some("application/json"),
                    material_json.as_bytes(),
                ),
            ],
        );
        let req = Request::builder()
            .method("POST")
            .uri("/validate/compatibility")
            .header(header::AUTHORIZATION, token)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

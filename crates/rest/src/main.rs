use axum::{
    extract::Multipart,
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use printproof3d_core::{MaterialProfile, PrinterProfile, ValidationReport};
use printproof3d_printability::{
    GcodeValidator, ModelValidator, StandardGcodeValidator, StlModelValidator,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use tower_http::cors::{AllowOrigin, CorsLayer};

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    let target_token = std::env::var("PRINTPROOF3D_API_TOKEN")
        .unwrap_or_else(|_| "secret_print_token".to_string());

    let expected_auth = format!("Bearer {}", target_token);

    if let Some(auth) = auth_header {
        if auth == expected_auth {
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

async fn home() -> &'static str {
    "PrintProof3D REST API"
}

async fn list_printer_profiles() -> Result<axum::Json<Vec<PrinterProfile>>, (StatusCode, String)> {
    let mut profiles_dir = std::env::current_dir().unwrap_or_default().join("profiles");
    if !profiles_dir.exists() {
        profiles_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("../../profiles");
    }

    if !profiles_dir.exists() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Profiles directory not found".to_string(),
        ));
    }

    let mut profiles = Vec::new();
    let entries = std::fs::read_dir(profiles_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    for entry_opt in entries {
        let entry = entry_opt.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            if let Ok(p) = serde_json::from_str::<PrinterProfile>(&contents) {
                if p.validate().is_ok() {
                    profiles.push(p);
                }
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
        .route("/", get(home))
        .route("/profiles/printers", get(list_printer_profiles))
        .route(
            "/validate/model",
            axum::routing::post(validate_model).route_layer(middleware::from_fn(auth_middleware)),
        )
        .route(
            "/validate/gcode",
            axum::routing::post(validate_gcode).route_layer(middleware::from_fn(auth_middleware)),
        )
        .layer(cors)
}

#[tokio::main]
async fn main() {
    let app = api_router();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
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
        assert_eq!(home().await, "PrintProof3D REST API");
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
}

use axum::{
    body::Body,
    extract::{Multipart, Request},
    http::{header, Response, StatusCode},
    middleware::{self, Next},
    routing::{get, post},
    Json, Router,
};
use tokio::sync::oneshot;

pub struct PrusaLinkMockServer {
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

fn parse_www_authenticate(header: &str) -> std::collections::HashMap<String, String> {
    let mut params = std::collections::HashMap::new();
    let s = header.strip_prefix("Digest ").unwrap_or(header);

    for item in s.split(',') {
        let mut parts = item.splitn(2, '=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            let key = k.trim().to_string();
            let value = v.trim().trim_matches('"').to_string();
            params.insert(key, value);
        }
    }
    params
}

fn md5_hex(data: &str) -> String {
    format!("{:x}", md5::compute(data))
}

#[allow(clippy::too_many_arguments)]
fn calculate_digest_response(
    username: &str,
    password: &str,
    method: &str,
    uri: &str,
    realm: &str,
    nonce: &str,
    qop: Option<&str>,
    nc: &str,
    cnonce: &str,
) -> String {
    let ha1 = md5_hex(&format!("{}:{}:{}", username, realm, password));
    let ha2 = md5_hex(&format!("{}:{}", method, uri));
    if let Some(q) = qop {
        md5_hex(&format!(
            "{}:{}:{}:{}:{}:{}",
            ha1, nonce, nc, cnonce, q, ha2
        ))
    } else {
        md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2))
    }
}

async fn digest_auth_middleware(req: Request, next: Next) -> Response<Body> {
    let auth_header = req.headers().get(header::AUTHORIZATION);

    let method = req.method().as_str();
    let uri = req.uri().path();

    let authenticated = if let Some(hdr) = auth_header {
        if let Ok(hdr_str) = hdr.to_str() {
            let params = parse_www_authenticate(hdr_str);
            if let (Some(username), Some(realm), Some(nonce), Some(response)) = (
                params.get("username"),
                params.get("realm"),
                params.get("nonce"),
                params.get("response"),
            ) {
                let qop = params.get("qop").map(|s| s.as_str());
                let nc = params.get("nc").cloned().unwrap_or_default();
                let cnonce = params.get("cnonce").cloned().unwrap_or_default();

                let expected_pass = "makerpass";
                let calculated = calculate_digest_response(
                    username,
                    expected_pass,
                    method,
                    uri,
                    realm,
                    nonce,
                    qop,
                    &nc,
                    &cnonce,
                );
                calculated == *response
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if authenticated {
        next.run(req).await
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(
                header::WWW_AUTHENTICATE,
                "Digest realm=\"PrusaLink\", nonce=\"testnonce\", qop=\"auth\", algorithm=MD5",
            )
            .body(Body::from("401 Unauthorized"))
            .unwrap()
    }
}

async fn handle_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "telemetry": {
            "temp-nozzle": 210.0,
            "target-nozzle": 210.0,
            "temp-bed": 60.0,
            "target-bed": 60.0,
            "state": "idle"
        }
    }))
}

async fn handle_upload(mut multipart: Multipart) -> Json<serde_json::Value> {
    while let Ok(Some(_field)) = multipart.next_field().await {}
    Json(serde_json::json!({
        "result": "uploaded"
    }))
}

async fn handle_job() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "result": "ok" }))
}

async fn handle_file_command() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "result": "ok" }))
}

impl PrusaLinkMockServer {
    pub fn start() -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let app = Router::new()
            .route("/api/v1/status", get(handle_status))
            .route("/api/v1/files/local", post(handle_upload))
            .route("/api/v1/files/local/:filename", post(handle_file_command))
            .route("/api/v1/job", post(handle_job))
            .route_layer(middleware::from_fn(digest_auth_middleware));

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();

        tokio::spawn(async move {
            axum::serve(tokio::net::TcpListener::from_std(listener).unwrap(), app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        PrusaLinkMockServer {
            port,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    pub fn get_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

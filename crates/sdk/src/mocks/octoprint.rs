use axum::{
    extract::{Multipart, Path},
    routing::{get, post},
    Json, Router,
};
use tokio::sync::oneshot;

pub struct OctoPrintMockServer {
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

async fn handle_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "temperature": {
            "tool0": {
                "actual": 210.0,
                "target": 210.0
            },
            "bed": {
                "actual": 60.0,
                "target": 60.0
            }
        },
        "state": {
            "text": "Operational",
            "flags": {
                "operational": true,
                "printing": false,
                "paused": false,
                "error": false
            }
        }
    }))
}

async fn handle_upload(mut multipart: Multipart) -> Json<serde_json::Value> {
    while let Ok(Some(_field)) = multipart.next_field().await {
        // Just consume multipart fields
    }
    Json(serde_json::json!({
        "files": {
            "local": {
                "name": "test.gcode",
                "origin": "local"
            }
        },
        "done": true
    }))
}

async fn handle_job_select(Path(_filename): Path<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "done": true }))
}

async fn handle_job_command() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "done": true }))
}

async fn handle_printer_command() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "done": true }))
}

impl OctoPrintMockServer {
    pub fn start() -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let app = Router::new()
            .route("/api/printer", get(handle_status))
            .route("/api/files/local", post(handle_upload))
            .route("/api/files/local/:filename", post(handle_job_select))
            .route("/api/job", post(handle_job_command))
            .route("/api/printer/command", post(handle_printer_command));

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

        OctoPrintMockServer {
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

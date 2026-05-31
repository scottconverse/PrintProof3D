use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        Multipart,
    },
    routing::{get, post},
    Json, Router,
};
use tokio::sync::oneshot;

pub struct MoonrakerMockServer {
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

async fn handle_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "result": {
            "state": "ready",
            "state_message": "Printer is ready"
        }
    }))
}

async fn handle_query() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "result": {
            "status": {
                "print_stats": {
                    "state": "ready"
                },
                "extruder": {
                    "temperature": 210.0,
                    "target": 210.0
                },
                "heater_bed": {
                    "temperature": 60.0,
                    "target": 60.0
                }
            }
        }
    }))
}

async fn handle_upload(mut multipart: Multipart) -> Json<serde_json::Value> {
    while let Ok(Some(_field)) = multipart.next_field().await {}
    Json(serde_json::json!({
        "result": {
            "item": {
                "path": "test_upload_conformance.gcode"
            }
        }
    }))
}

async fn handle_start() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "result": "ok" }))
}

async fn handle_pause() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "result": "ok" }))
}

async fn handle_resume() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "result": "ok" }))
}

async fn handle_cancel() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "result": "ok" }))
}

async fn handle_gcode() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "result": "ok" }))
}

async fn handle_ws(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let telemetry = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notify_status_update",
            "params": [{
                "print_stats": {
                    "state": "ready"
                },
                "extruder": {
                    "temperature": 210.0,
                    "target": 210.0
                },
                "heater_bed": {
                    "temperature": 60.0,
                    "target": 60.0
                }
            }]
        });
        if socket
            .send(Message::Text(telemetry.to_string()))
            .await
            .is_err()
        {
            return;
        }

        while let Some(Ok(msg)) = socket.recv().await {
            match msg {
                Message::Text(txt) => {
                    if txt.contains("printer.objects.subscribe") {
                        let response = serde_json::json!({
                            "jsonrpc": "2.0",
                            "result": {
                                "status": {
                                    "print_stats": { "state": "ready" },
                                    "extruder": { "temperature": 210.0, "target": 210.0 },
                                    "heater_bed": { "temperature": 60.0, "target": 60.0 }
                                }
                            },
                            "id": 1
                        });
                        let _ = socket.send(Message::Text(response.to_string())).await;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    })
}

impl MoonrakerMockServer {
    pub fn start() -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let app = Router::new()
            .route("/printer/info", get(handle_info))
            .route("/printer/objects/query", get(handle_query))
            .route("/server/files/upload", post(handle_upload))
            .route("/printer/print/start", post(handle_start))
            .route("/printer/print/pause", post(handle_pause))
            .route("/printer/print/resume", post(handle_resume))
            .route("/printer/print/cancel", post(handle_cancel))
            .route("/printer/gcode/script", post(handle_gcode))
            .route("/websocket", get(handle_ws));

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

        MoonrakerMockServer {
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

// Klipper Moonraker Printer Adapter
use crate::{AdapterError, PrinterAdapter, PrinterState, PrinterTelemetry};
use async_trait::async_trait;
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile};
use std::path::Path;

#[allow(dead_code)]
pub struct MoonrakerAdapter {
    profile: PrinterProfile,
    config: PrinterConnectionConfig,
    client: reqwest::Client,
}

impl MoonrakerAdapter {
    pub fn new(profile: PrinterProfile, config: PrinterConnectionConfig) -> Self {
        Self {
            profile,
            config,
            client: reqwest::Client::new(),
        }
    }

    fn get_api_key(&self) -> Result<Option<String>, AdapterError> {
        if self.config.auth_type == printproof3d_core::connection::AuthType::ApiKey {
            let env_var = self.config.api_key_env_var.as_deref().ok_or_else(|| {
                AdapterError::AuthenticationFailed(
                    "API key environment variable name is not configured".to_string(),
                )
            })?;
            let api_key = std::env::var(env_var).map_err(|_| {
                AdapterError::AuthenticationFailed(format!(
                    "Environment variable {} is not set",
                    env_var
                ))
            })?;
            if api_key.trim().is_empty() {
                return Err(AdapterError::AuthenticationFailed(format!(
                    "Environment variable {} is empty",
                    env_var
                )));
            }
            Ok(Some(api_key))
        } else {
            Ok(None)
        }
    }

    fn check_dispatch_upload(&self) -> Result<(), AdapterError> {
        if self.config.dispatch_policy == printproof3d_core::connection::DispatchPolicy::DryRunOnly
        {
            return Err(AdapterError::UploadFailed(
                "Operation disallowed by DispatchPolicy::DryRunOnly".to_string(),
            ));
        }
        Ok(())
    }

    fn check_dispatch_control(&self) -> Result<(), AdapterError> {
        match self.config.dispatch_policy {
            printproof3d_core::connection::DispatchPolicy::DryRunOnly => {
                Err(AdapterError::CommandFailed(
                    "Operation disallowed by DispatchPolicy::DryRunOnly".to_string(),
                ))
            }
            printproof3d_core::connection::DispatchPolicy::UploadOnly => {
                Err(AdapterError::CommandFailed(
                    "Operation disallowed by DispatchPolicy::UploadOnly".to_string(),
                ))
            }
            printproof3d_core::connection::DispatchPolicy::AllowStart => Ok(()),
        }
    }

    async fn post_json(&self, path: &str, body: serde_json::Value) -> Result<(), AdapterError> {
        let key = self.get_api_key()?;
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::CommandFailed("Base URL is missing".to_string()))?;
        let url = format!("{}{}", base_url, path);
        let mut builder = self.client.post(&url);
        if let Some(ref k) = key {
            builder = builder.header("X-Api-Key", k);
        }
        let resp = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "Moonraker POST {} failed: {}",
                path,
                resp.status()
            )))
        }
    }
}

#[async_trait]
impl PrinterAdapter for MoonrakerAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError> {
        let key = self.get_api_key()?;
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::ConnectionFailed("Base URL is missing".to_string()))?;

        // 1. Check REST API
        let info_url = format!("{}/printer/info", base_url);
        let mut builder = self.client.get(&info_url);
        if let Some(ref k) = key {
            builder = builder.header("X-Api-Key", k);
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AdapterError::ConnectionFailed(format!(
                "Moonraker connection check failed: {}",
                resp.status()
            )));
        }

        // 2. Check WebSocket API using tokio-tungstenite
        let mut ws_url =
            url::Url::parse(base_url).map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;
        let scheme = match ws_url.scheme() {
            "https" => "wss",
            _ => "ws",
        };
        ws_url
            .set_scheme(scheme)
            .map_err(|_| AdapterError::ConnectionFailed("Failed to set scheme".to_string()))?;
        ws_url.set_path("/websocket");
        if let Some(ref k) = key {
            ws_url.query_pairs_mut().append_pair("token", k);
        }

        let (mut ws_stream, _) = tokio_tungstenite::connect_async(ws_url.as_str())
            .await
            .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;

        let _ = ws_stream.close(None).await;

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
        let key = self.get_api_key()?;
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::CommandFailed("Base URL is missing".to_string()))?;
        let query_url = format!(
            "{}/printer/objects/query?print_stats&extruder&heater_bed",
            base_url
        );
        let mut builder = self.client.get(&query_url);
        if let Some(ref k) = key {
            builder = builder.header("X-Api-Key", k);
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AdapterError::CommandFailed(format!(
                "Moonraker query failed: {}",
                resp.status()
            )));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        let status = json.get("result").and_then(|r| r.get("status"));
        let state_str = status
            .and_then(|s| s.get("print_stats"))
            .and_then(|p| p.get("state"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let state = match state_str {
            "printing" => PrinterState::Printing,
            "paused" => PrinterState::Paused,
            "ready" | "standby" | "complete" => PrinterState::Idle,
            "error" => PrinterState::Error,
            _ => PrinterState::Unknown,
        };

        let tool_temp = status
            .and_then(|s| s.get("extruder"))
            .and_then(|e| e.get("temperature"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let tool_target = status
            .and_then(|s| s.get("extruder"))
            .and_then(|e| e.get("target"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let bed_temp = status
            .and_then(|s| s.get("heater_bed"))
            .and_then(|b| b.get("temperature"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let bed_target = status
            .and_then(|s| s.get("heater_bed"))
            .and_then(|b| b.get("target"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        Ok(PrinterTelemetry {
            state,
            tool_temp,
            tool_target,
            bed_temp,
            bed_target,
            progress: 0.0,
            current_file: None,
        })
    }

    async fn upload_file(
        &self,
        local_path: &Path,
        remote_name: &str,
    ) -> Result<String, AdapterError> {
        self.check_dispatch_upload()?;
        let key = self.get_api_key()?;
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::UploadFailed("Base URL is missing".to_string()))?;
        let url = format!("{}/server/files/upload", base_url);

        let file_content = tokio::fs::read(local_path)
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        let part = reqwest::multipart::Part::bytes(file_content).file_name(remote_name.to_string());
        let form = reqwest::multipart::Form::new().part("file", part);

        let mut builder = self.client.post(&url);
        if let Some(ref k) = key {
            builder = builder.header("X-Api-Key", k);
        }
        let resp = builder
            .multipart(form)
            .send()
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(remote_name.to_string())
        } else {
            Err(AdapterError::UploadFailed(format!(
                "Moonraker upload failed: {}",
                resp.status()
            )))
        }
    }

    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let key = self.get_api_key()?;
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::CommandFailed("Base URL is missing".to_string()))?;
        let url = format!("{}/printer/print/start?filename={}", base_url, file_id);
        let mut builder = self.client.post(&url);
        if let Some(ref k) = key {
            builder = builder.header("X-Api-Key", k);
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "Moonraker start job failed: {}",
                resp.status()
            )))
        }
    }

    async fn pause_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.post_json("/printer/print/pause", serde_json::json!({}))
            .await
    }

    async fn resume_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.post_json("/printer/print/resume", serde_json::json!({}))
            .await
    }

    async fn cancel_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.post_json("/printer/print/cancel", serde_json::json!({}))
            .await
    }

    async fn emergency_stop(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.post_json(
            "/printer/gcode/script",
            serde_json::json!({
                "script": "M112"
            }),
        )
        .await
    }
}

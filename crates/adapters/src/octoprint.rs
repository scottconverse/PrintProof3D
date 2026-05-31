// OctoPrint Printer Adapter
use crate::{AdapterError, PrinterAdapter, PrinterState, PrinterTelemetry};
use async_trait::async_trait;
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile};
use std::path::Path;

#[allow(dead_code)]
pub struct OctoPrintAdapter {
    profile: PrinterProfile,
    config: PrinterConnectionConfig,
    client: reqwest::Client,
}

impl OctoPrintAdapter {
    pub fn new(profile: PrinterProfile, config: PrinterConnectionConfig) -> Self {
        Self {
            profile,
            config,
            client: reqwest::Client::new(),
        }
    }

    fn get_client_and_key(&self) -> Result<(reqwest::Client, String), AdapterError> {
        let env_var = self
            .config
            .api_key_env_var
            .as_deref()
            .unwrap_or("OCTOPRINT_API_KEY");
        let api_key = std::env::var(env_var).unwrap_or_default();
        Ok((self.client.clone(), api_key))
    }

    async fn post_json(&self, path: &str, body: serde_json::Value) -> Result<(), AdapterError> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::CommandFailed("Base URL is missing".to_string()))?;
        let (client, api_key) = self.get_client_and_key()?;
        let url = format!("{}{}", base_url, path);
        let resp = client
            .post(&url)
            .header("X-Api-Key", &api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "OctoPrint POST {} failed: {}",
                path,
                resp.status()
            )))
        }
    }
}

#[async_trait]
impl PrinterAdapter for OctoPrintAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::ConnectionFailed("Base URL is missing".to_string()))?;
        let (client, api_key) = self.get_client_and_key()?;
        let url = format!("{}/api/printer", base_url);
        let resp = client
            .get(&url)
            .header("X-Api-Key", &api_key)
            .send()
            .await
            .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::ConnectionFailed(format!(
                "OctoPrint connect failed with status: {}",
                resp.status()
            )))
        }
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::CommandFailed("Base URL is missing".to_string()))?;
        let (client, api_key) = self.get_client_and_key()?;
        let url = format!("{}/api/printer", base_url);
        let resp = client
            .get(&url)
            .header("X-Api-Key", &api_key)
            .send()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AdapterError::CommandFailed(format!(
                "OctoPrint status request failed: {}",
                resp.status()
            )));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        let state_flags = json.get("state").and_then(|s| s.get("flags"));
        let state = if let Some(flags) = state_flags {
            if flags
                .get("error")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                PrinterState::Error
            } else if flags
                .get("printing")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                PrinterState::Printing
            } else if flags
                .get("paused")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                PrinterState::Paused
            } else if flags
                .get("operational")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                PrinterState::Idle
            } else {
                PrinterState::Unknown
            }
        } else {
            PrinterState::Unknown
        };

        let temps = json.get("temperature");
        let tool_temp = temps
            .and_then(|t| t.get("tool0"))
            .and_then(|h| h.get("actual"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let tool_target = temps
            .and_then(|t| t.get("tool0"))
            .and_then(|h| h.get("target"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let bed_temp = temps
            .and_then(|t| t.get("bed"))
            .and_then(|h| h.get("actual"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let bed_target = temps
            .and_then(|t| t.get("bed"))
            .and_then(|h| h.get("target"))
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
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::UploadFailed("Base URL is missing".to_string()))?;
        let (client, api_key) = self.get_client_and_key()?;
        let url = format!("{}/api/files/local", base_url);

        let file_content = tokio::fs::read(local_path)
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        let part = reqwest::multipart::Part::bytes(file_content).file_name(remote_name.to_string());
        let form = reqwest::multipart::Form::new().part("file", part);

        let resp = client
            .post(&url)
            .header("X-Api-Key", &api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(remote_name.to_string())
        } else {
            Err(AdapterError::UploadFailed(format!(
                "OctoPrint upload failed: {}",
                resp.status()
            )))
        }
    }

    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError> {
        let path = format!("/api/files/local/{}", file_id);
        self.post_json(
            &path,
            serde_json::json!({
                "command": "select",
                "print": true
            }),
        )
        .await
    }

    async fn pause_job(&self) -> Result<(), AdapterError> {
        self.post_json(
            "/api/job",
            serde_json::json!({
                "command": "pause",
                "action": "pause"
            }),
        )
        .await
    }

    async fn resume_job(&self) -> Result<(), AdapterError> {
        self.post_json(
            "/api/job",
            serde_json::json!({
                "command": "pause",
                "action": "resume"
            }),
        )
        .await
    }

    async fn cancel_job(&self) -> Result<(), AdapterError> {
        self.post_json(
            "/api/job",
            serde_json::json!({
                "command": "cancel"
            }),
        )
        .await
    }

    async fn emergency_stop(&self) -> Result<(), AdapterError> {
        self.post_json(
            "/api/printer/command",
            serde_json::json!({
                "command": "M112"
            }),
        )
        .await
    }
}

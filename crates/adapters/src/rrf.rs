// RepRapFirmware Printer Adapter
use crate::{AdapterError, PrinterAdapter, PrinterState, PrinterTelemetry};
use async_trait::async_trait;
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile};
use std::path::Path;

#[allow(dead_code)]
pub struct RrfAdapter {
    profile: PrinterProfile,
    config: PrinterConnectionConfig,
    client: reqwest::Client,
}

impl RrfAdapter {
    pub fn new(profile: PrinterProfile, config: PrinterConnectionConfig) -> Self {
        Self {
            profile,
            config,
            client: reqwest::Client::new(),
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

    async fn send_gcode(&self, gcode: &str) -> Result<(), AdapterError> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::CommandFailed("Base URL is missing".to_string()))?;
        let encoded: String = url::form_urlencoded::byte_serialize(gcode.as_bytes()).collect();
        let url = format!("{}/rr_gcode?gcode={}", base_url, encoded);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "RRF G-code command failed: {}",
                resp.status()
            )))
        }
    }
}

#[async_trait]
impl PrinterAdapter for RrfAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::ConnectionFailed("Base URL is missing".to_string()))?;
        let url = format!("{}/rr_connect", base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::ConnectionFailed(format!(
                "RRF connect failed with status: {}",
                resp.status()
            )))
        }
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::ConnectionFailed("Base URL is missing".to_string()))?;
        let url = format!("{}/rr_disconnect", base_url);
        let _ = self.client.get(&url).send().await;
        Ok(())
    }

    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::CommandFailed("Base URL is missing".to_string()))?;
        let url = format!("{}/rr_status", base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AdapterError::CommandFailed(format!(
                "RRF status request failed: {}",
                resp.status()
            )));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        let state_str = json
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let state = match state_str {
            "I" | "idle" | "IDLE" => PrinterState::Idle,
            "P" | "printing" | "PRINTING" => PrinterState::Printing,
            "paused" | "PAUSED" => PrinterState::Paused,
            "error" | "ERROR" => PrinterState::Error,
            _ => PrinterState::Unknown,
        };

        let tool_temp = json
            .get("temps")
            .and_then(|t| t.get("heads"))
            .and_then(|h| h.get(0))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let bed_temp = json
            .get("temps")
            .and_then(|t| t.get("bed"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        Ok(PrinterTelemetry {
            state,
            tool_temp,
            tool_target: tool_temp,
            bed_temp,
            bed_target: bed_temp,
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
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::UploadFailed("Base URL is missing".to_string()))?;
        let url = format!("{}/rr_upload?name={}", base_url, remote_name);

        let file_content = tokio::fs::read(local_path)
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        let resp = self
            .client
            .post(&url)
            .body(file_content)
            .send()
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(remote_name.to_string())
        } else {
            Err(AdapterError::UploadFailed(format!(
                "RRF upload failed: {}",
                resp.status()
            )))
        }
    }

    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.send_gcode(&format!("M32 {}", file_id)).await
    }

    async fn pause_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.send_gcode("M25").await
    }

    async fn resume_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.send_gcode("M24").await
    }

    async fn cancel_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.send_gcode("M0").await
    }

    async fn emergency_stop(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        self.send_gcode("M112").await
    }
}

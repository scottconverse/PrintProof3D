// PrusaLink Printer Adapter
use crate::{AdapterError, PrinterAdapter, PrinterState, PrinterTelemetry};
use async_trait::async_trait;
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile};
use std::path::Path;

#[allow(dead_code)]
pub struct PrusaLinkAdapter {
    profile: PrinterProfile,
    config: PrinterConnectionConfig,
    client: reqwest::Client,
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

impl PrusaLinkAdapter {
    pub fn new(profile: PrinterProfile, config: PrinterConnectionConfig) -> Self {
        Self {
            profile,
            config,
            client: reqwest::Client::new(),
        }
    }

    fn get_credentials(&self) -> Result<(String, String), AdapterError> {
        if self.config.auth_type == printproof3d_core::connection::AuthType::Digest
            || self.config.auth_type == printproof3d_core::connection::AuthType::Password
        {
            let username = self
                .config
                .username
                .clone()
                .ok_or_else(|| AdapterError::AuthenticationFailed("Username is not configured".to_string()))?;
            if username.trim().is_empty() {
                return Err(AdapterError::AuthenticationFailed("Username is empty".to_string()));
            }
            let env_var = self
                .config
                .password_env_var
                .as_deref()
                .ok_or_else(|| AdapterError::AuthenticationFailed("Password environment variable name is not configured".to_string()))?;
            let password = std::env::var(env_var).map_err(|_| {
                AdapterError::AuthenticationFailed(format!("Environment variable {} is not set", env_var))
            })?;
            if password.trim().is_empty() {
                return Err(AdapterError::AuthenticationFailed(format!("Environment variable {} is empty", env_var)));
            }
            Ok((username, password))
        } else {
            let username = self.config.username.clone().unwrap_or_else(|| "maker".to_string());
            let env_var = self.config.password_env_var.as_deref().unwrap_or("PRUSALINK_PASSWORD");
            let password = std::env::var(env_var).unwrap_or_else(|_| "makerpass".to_string());
            Ok((username, password))
        }
    }

    fn check_dispatch_upload(&self) -> Result<(), AdapterError> {
        if self.config.dispatch_policy == printproof3d_core::connection::DispatchPolicy::DryRunOnly {
            return Err(AdapterError::UploadFailed("Operation disallowed by DispatchPolicy::DryRunOnly".to_string()));
        }
        Ok(())
    }

    fn check_dispatch_control(&self) -> Result<(), AdapterError> {
        match self.config.dispatch_policy {
            printproof3d_core::connection::DispatchPolicy::DryRunOnly => {
                Err(AdapterError::CommandFailed("Operation disallowed by DispatchPolicy::DryRunOnly".to_string()))
            }
            printproof3d_core::connection::DispatchPolicy::UploadOnly => {
                Err(AdapterError::CommandFailed("Operation disallowed by DispatchPolicy::UploadOnly".to_string()))
            }
            printproof3d_core::connection::DispatchPolicy::AllowStart => Ok(()),
        }
    }

    async fn send_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::Response, AdapterError> {
        let _ = self.get_credentials()?;
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::CommandFailed("Base URL is missing".to_string()))?;
        let url = format!("{}{}", base_url, path);
        let client = self.client.clone();

        // 1. Send request without auth
        let mut builder = client.request(method.clone(), &url);
        if let Some(ref b) = body {
            builder = builder.json(b);
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            // Parse challenge
            if let Some(auth_header) = resp.headers().get(reqwest::header::WWW_AUTHENTICATE) {
                let auth_str = auth_header
                    .to_str()
                    .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;
                let params = parse_www_authenticate(auth_str);

                let realm = params.get("realm").cloned().unwrap_or_default();
                let nonce = params.get("nonce").cloned().unwrap_or_default();
                let qop = params.get("qop").cloned();

                let (username, password) = self.get_credentials()?;
                let nc = "00000001";
                let cnonce = "clientnonce";

                let digest_response = calculate_digest_response(
                    &username,
                    &password,
                    method.as_str(),
                    path,
                    &realm,
                    &nonce,
                    qop.as_deref(),
                    nc,
                    cnonce,
                );

                let mut auth_val = format!(
                    "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\"",
                    username, realm, nonce, path, digest_response
                );
                if let Some(ref q) = qop {
                    auth_val.push_str(&format!(", qop={}, nc={}, cnonce=\"{}\"", q, nc, cnonce));
                }

                // Retry request with auth header
                let mut retry_builder = client
                    .request(method, &url)
                    .header(reqwest::header::AUTHORIZATION, auth_val);
                if let Some(ref b) = body {
                    retry_builder = retry_builder.json(b);
                }

                let retry_resp = retry_builder
                    .send()
                    .await
                    .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;
                Ok(retry_resp)
            } else {
                Err(AdapterError::CommandFailed(
                    "Missing WWW-Authenticate header".to_string(),
                ))
            }
        } else {
            Ok(resp)
        }
    }
}

#[async_trait]
impl PrinterAdapter for PrusaLinkAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError> {
        let resp = self
            .send_request(reqwest::Method::GET, "/api/v1/status", None)
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::ConnectionFailed(format!(
                "PrusaLink connect failed: {}",
                resp.status()
            )))
        }
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
        let resp = self
            .send_request(reqwest::Method::GET, "/api/v1/status", None)
            .await?;
        if !resp.status().is_success() {
            return Err(AdapterError::CommandFailed(format!(
                "PrusaLink status query failed: {}",
                resp.status()
            )));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        let tel = json.get("telemetry");
        let state_str = tel
            .and_then(|t| t.get("state"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let state = match state_str {
            "printing" => PrinterState::Printing,
            "paused" => PrinterState::Paused,
            "idle" => PrinterState::Idle,
            "error" => PrinterState::Error,
            _ => PrinterState::Unknown,
        };

        let tool_temp = tel
            .and_then(|t| t.get("temp-nozzle"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let tool_target = tel
            .and_then(|t| t.get("target-nozzle"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let bed_temp = tel
            .and_then(|t| t.get("temp-bed"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let bed_target = tel
            .and_then(|t| t.get("target-bed"))
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
        let _ = self.get_credentials()?;
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::UploadFailed("Base URL is missing".to_string()))?;
        let url = format!("{}{}", base_url, "/api/v1/files/local");
        let client = self.client.clone();

        // 1. Read file
        let file_content = tokio::fs::read(local_path)
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        // 2. Perform the challenge-response handshaking for POST
        // First send a dummy request to /api/v1/files/local to trigger 401 challenge
        let init_resp = client
            .post(&url)
            .send()
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        if init_resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            if let Some(auth_header) = init_resp.headers().get(reqwest::header::WWW_AUTHENTICATE) {
                let auth_str = auth_header
                    .to_str()
                    .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;
                let params = parse_www_authenticate(auth_str);

                let realm = params.get("realm").cloned().unwrap_or_default();
                let nonce = params.get("nonce").cloned().unwrap_or_default();
                let qop = params.get("qop").cloned();

                let (username, password) = self.get_credentials()?;
                let nc = "00000001";
                let cnonce = "clientnonce";

                let digest_response = calculate_digest_response(
                    &username,
                    &password,
                    "POST",
                    "/api/v1/files/local",
                    &realm,
                    &nonce,
                    qop.as_deref(),
                    nc,
                    cnonce,
                );

                let mut auth_val = format!(
                    "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"/api/v1/files/local\", response=\"{}\"",
                    username, realm, nonce, digest_response
                );
                if let Some(ref q) = qop {
                    auth_val.push_str(&format!(", qop={}, nc={}, cnonce=\"{}\"", q, nc, cnonce));
                }

                // Send actual upload request with auth header
                let part = reqwest::multipart::Part::bytes(file_content)
                    .file_name(remote_name.to_string());
                let form = reqwest::multipart::Form::new().part("file", part);

                let resp = client
                    .post(&url)
                    .header(reqwest::header::AUTHORIZATION, auth_val)
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

                if resp.status().is_success() {
                    Ok(remote_name.to_string())
                } else {
                    Err(AdapterError::UploadFailed(format!(
                        "PrusaLink upload failed: {}",
                        resp.status()
                    )))
                }
            } else {
                Err(AdapterError::UploadFailed(
                    "Missing WWW-Authenticate header".to_string(),
                ))
            }
        } else {
            Err(AdapterError::UploadFailed(
                "Expected 401 auth challenge".to_string(),
            ))
        }
    }

    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let path = format!("/api/v1/files/local/{}", file_id);
        let resp = self
            .send_request(
                reqwest::Method::POST,
                &path,
                Some(serde_json::json!({
                    "command": "start"
                })),
            )
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "PrusaLink start job failed: {}",
                resp.status()
            )))
        }
    }

    async fn pause_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let resp = self
            .send_request(
                reqwest::Method::POST,
                "/api/v1/job",
                Some(serde_json::json!({
                    "command": "pause"
                })),
            )
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "PrusaLink pause job failed: {}",
                resp.status()
            )))
        }
    }

    async fn resume_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let resp = self
            .send_request(
                reqwest::Method::POST,
                "/api/v1/job",
                Some(serde_json::json!({
                    "command": "resume"
                })),
            )
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "PrusaLink resume job failed: {}",
                resp.status()
            )))
        }
    }

    async fn cancel_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let resp = self
            .send_request(
                reqwest::Method::POST,
                "/api/v1/job",
                Some(serde_json::json!({
                    "command": "cancel"
                })),
            )
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "PrusaLink cancel job failed: {}",
                resp.status()
            )))
        }
    }

    async fn emergency_stop(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let resp = self
            .send_request(
                reqwest::Method::POST,
                "/api/v1/job",
                Some(serde_json::json!({
                    "command": "cancel"
                })),
            )
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AdapterError::CommandFailed(format!(
                "PrusaLink emergency stop failed: {}",
                resp.status()
            )))
        }
    }
}

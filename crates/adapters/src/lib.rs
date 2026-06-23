// PrintProof3D Adapters Crate
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrinterState {
    Idle,
    Printing,
    Paused,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdapterError {
    ConnectionFailed(String),
    AuthenticationFailed(String),
    UploadFailed(String),
    CommandFailed(String),
    ValidationError(String),
    Timeout,
    Unknown(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            AdapterError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            AdapterError::UploadFailed(msg) => write!(f, "Upload failed: {}", msg),
            AdapterError::CommandFailed(msg) => write!(f, "Command failed: {}", msg),
            AdapterError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AdapterError::Timeout => write!(f, "Timeout"),
            AdapterError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl Error for AdapterError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrinterTelemetry {
    pub state: PrinterState,
    pub tool_temp: f32,
    pub tool_target: f32,
    pub bed_temp: f32,
    pub bed_target: f32,
    pub progress: f32,
    pub current_file: Option<String>,
}

#[async_trait]
pub trait PrinterAdapter: Send + Sync {
    async fn connect(&mut self) -> Result<(), AdapterError>;
    async fn disconnect(&mut self) -> Result<(), AdapterError>;
    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError>;
    async fn upload_file(
        &self,
        local_path: &Path,
        remote_name: &str,
    ) -> Result<String, AdapterError>;
    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError>;
    async fn pause_job(&self) -> Result<(), AdapterError>;
    async fn resume_job(&self) -> Result<(), AdapterError>;
    async fn cancel_job(&self) -> Result<(), AdapterError>;
    async fn emergency_stop(&self) -> Result<(), AdapterError>;
}

/// Connection (handshake) timeout for HTTP-based printer adapters.
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 10;
/// Overall per-request timeout so an unresponsive or black-holed printer cannot hang an adapter
/// indefinitely. The serial adapter already enforces its own read timeout; this brings the HTTP
/// adapters (Moonraker, OctoPrint, PrusaLink, RepRapFirmware) in line.
const HTTP_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Builds a `reqwest::Client` with sane connect/request timeouts shared by all HTTP adapters.
/// Falls back to a default client if the builder fails (e.g. TLS backend init), preserving the
/// previous best-effort construction behavior rather than panicking at adapter creation.
pub(crate) fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(HTTP_REQUEST_TIMEOUT_SECS))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

pub mod bambu;
pub mod factory;
pub mod moonraker;
pub mod octoprint;
pub mod prusalink;
pub mod rrf;
pub mod serial;

#[cfg(test)]
mod tests_dispatch_policy;

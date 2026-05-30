// PrintProof3D Adapters Crate
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::path::Path;

pub fn list_adapters() -> Vec<&'static str> {
    vec!["moonraker", "octoprint", "marlin"]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdapterError {
    ConnectionFailed(String),
    AuthenticationFailed(String),
    UploadFailed(String),
    CommandFailed(String),
    Timeout,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrinterTelemetry {
    pub state: String,
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
    async fn upload_file(&self, local_path: &Path, remote_name: &str) -> Result<String, AdapterError>;
    async fn start_job(&self, file_id: &str) -> Result<(), AdapterError>;
    async fn pause_job(&self) -> Result<(), AdapterError>;
    async fn resume_job(&self) -> Result<(), AdapterError>;
    async fn cancel_job(&self) -> Result<(), AdapterError>;
    async fn emergency_stop(&self) -> Result<(), AdapterError>;
}

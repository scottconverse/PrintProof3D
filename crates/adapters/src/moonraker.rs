// Klipper Moonraker Printer Adapter
use crate::{AdapterError, PrinterAdapter, PrinterTelemetry};
use async_trait::async_trait;
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile};
use std::path::Path;

#[allow(dead_code)]
pub struct MoonrakerAdapter {
    profile: PrinterProfile,
    config: PrinterConnectionConfig,
}

impl MoonrakerAdapter {
    pub fn new(profile: PrinterProfile, config: PrinterConnectionConfig) -> Self {
        Self { profile, config }
    }
}

#[async_trait]
impl PrinterAdapter for MoonrakerAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError> {
        Err(AdapterError::ConnectionFailed(
            "Not implemented".to_string(),
        ))
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        Err(AdapterError::CommandFailed("Not implemented".to_string()))
    }

    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
        Err(AdapterError::CommandFailed("Not implemented".to_string()))
    }

    async fn upload_file(
        &self,
        _local_path: &Path,
        _remote_name: &str,
    ) -> Result<String, AdapterError> {
        Err(AdapterError::UploadFailed("Not implemented".to_string()))
    }

    async fn start_job(&self, _file_id: &str) -> Result<(), AdapterError> {
        Err(AdapterError::CommandFailed("Not implemented".to_string()))
    }

    async fn pause_job(&self) -> Result<(), AdapterError> {
        Err(AdapterError::CommandFailed("Not implemented".to_string()))
    }

    async fn resume_job(&self) -> Result<(), AdapterError> {
        Err(AdapterError::CommandFailed("Not implemented".to_string()))
    }

    async fn cancel_job(&self) -> Result<(), AdapterError> {
        Err(AdapterError::CommandFailed("Not implemented".to_string()))
    }

    async fn emergency_stop(&self) -> Result<(), AdapterError> {
        Err(AdapterError::CommandFailed("Not implemented".to_string()))
    }
}

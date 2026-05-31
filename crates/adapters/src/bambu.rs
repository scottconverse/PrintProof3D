// Bambu Lab Printer Adapter
use crate::{AdapterError, PrinterAdapter, PrinterState, PrinterTelemetry};
use async_trait::async_trait;
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile};
use std::path::Path;

#[allow(dead_code)]
pub struct BambuAdapter {
    profile: PrinterProfile,
    config: PrinterConnectionConfig,
}

impl BambuAdapter {
    pub fn new(profile: PrinterProfile, config: PrinterConnectionConfig) -> Self {
        Self { profile, config }
    }
}

#[async_trait]
impl PrinterAdapter for BambuAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError> {
        Err(AdapterError::ConnectionFailed(
            "Not implemented".to_string(),
        ))
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
        Ok(PrinterTelemetry {
            state: PrinterState::Idle,
            tool_temp: 0.0,
            tool_target: 0.0,
            bed_temp: 0.0,
            bed_target: 0.0,
            progress: 0.0,
            current_file: None,
        })
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

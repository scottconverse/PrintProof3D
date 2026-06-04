// Bambu Lab Printer Adapter
use crate::{AdapterError, PrinterAdapter, PrinterState, PrinterTelemetry};
use async_trait::async_trait;
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
pub struct BambuAdapter {
    profile: PrinterProfile,
    config: PrinterConnectionConfig,
    client: Option<AsyncClient>,
    telemetry: Arc<Mutex<PrinterTelemetry>>,
}

impl BambuAdapter {
    pub fn new(profile: PrinterProfile, config: PrinterConnectionConfig) -> Self {
        Self {
            profile,
            config,
            client: None,
            telemetry: Arc::new(Mutex::new(PrinterTelemetry {
                state: PrinterState::Unknown,
                tool_temp: 0.0,
                tool_target: 0.0,
                bed_temp: 0.0,
                bed_target: 0.0,
                progress: 0.0,
                current_file: None,
            })),
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
}

#[async_trait]
impl PrinterAdapter for BambuAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError> {
        let base_url = self.config.base_url.as_deref().unwrap_or("127.0.0.1");
        let parts: Vec<&str> = base_url.split(':').collect();
        let host = parts.first().copied().unwrap_or("127.0.0.1");
        let mqtt_port = parts
            .get(1)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(1883);

        let mut mqttoptions = MqttOptions::new("bambu_client", host, mqtt_port);
        mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        let telemetry_clone = self.telemetry.clone();
        let client_clone = client.clone();

        tokio::spawn(async move {
            let _ = client_clone
                .subscribe("device/+/report", QoS::AtMostOnce)
                .await;
            let _ = client_clone.subscribe("test", QoS::AtMostOnce).await;

            while let Ok(notification) = eventloop.poll().await {
                if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
                    if let Ok(payload) = String::from_utf8(publish.payload.to_vec()) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) {
                            if let Some(print) = json.get("print") {
                                let gcode_state = print
                                    .get("gcode_state")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                let state = match gcode_state {
                                    "RUNNING" | "printing" => PrinterState::Printing,
                                    "PAUSE" | "paused" => PrinterState::Paused,
                                    "IDLE" | "idle" | "ready" => PrinterState::Idle,
                                    "FAILED" | "error" => PrinterState::Error,
                                    _ => PrinterState::Unknown,
                                };
                                let tool_temp = print
                                    .get("nozzle_temper")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0)
                                    as f32;
                                let bed_temp = print
                                    .get("bed_temper")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0)
                                    as f32;

                                let mut guard = telemetry_clone.lock().unwrap();
                                guard.state = state;
                                guard.tool_temp = tool_temp;
                                guard.tool_target = tool_temp;
                                guard.bed_temp = bed_temp;
                                guard.bed_target = bed_temp;
                            }
                        }
                    }
                }
            }
        });

        // Wait a short time for MQTT connection handshake
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        self.client = Some(client);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        self.client = None;
        Ok(())
    }

    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
        let guard = self.telemetry.lock().unwrap();
        Ok(guard.clone())
    }

    async fn upload_file(
        &self,
        local_path: &Path,
        remote_name: &str,
    ) -> Result<String, AdapterError> {
        self.check_dispatch_upload()?;
        let base_url = self.config.base_url.as_deref().unwrap_or("127.0.0.1");
        let parts: Vec<&str> = base_url.split(':').collect();
        let host = parts.first().copied().unwrap_or("127.0.0.1").to_string();
        let ftp_port = parts
            .get(2)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(21);

        let local_path = local_path.to_owned();
        let remote_name = remote_name.to_string();

        tokio::task::spawn_blocking(move || {
            use std::io::Cursor;
            use suppaftp::FtpStream;
            let mut ftp = FtpStream::connect(format!("{}:{}", host, ftp_port))
                .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

            ftp.login("bambu", "bambu")
                .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

            let file_data = std::fs::read(&local_path)
                .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

            let mut cursor = Cursor::new(file_data);
            ftp.put_file(&remote_name, &mut cursor)
                .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

            let _ = ftp.quit();
            Ok::<String, AdapterError>(remote_name)
        })
        .await
        .map_err(|e| AdapterError::UploadFailed(e.to_string()))?
    }

    async fn start_job(&self, _file_id: &str) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        Ok(())
    }

    async fn pause_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        Ok(())
    }

    async fn resume_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        Ok(())
    }

    async fn cancel_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        Ok(())
    }

    async fn emergency_stop(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        Ok(())
    }
}

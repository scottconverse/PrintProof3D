// Marlin Serial Printer Adapter
use crate::{AdapterError, PrinterAdapter, PrinterState, PrinterTelemetry};
use async_trait::async_trait;
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

// Virtual mock serial stream defined inside the adapter crate to avoid circular dependency.
#[derive(Clone, Default)]
struct MarlinMockStream {
    input: Arc<Mutex<Vec<u8>>>,
    output: Arc<Mutex<Vec<u8>>>,
}

impl MarlinMockStream {
    fn new() -> Self {
        Self {
            input: Arc::new(Mutex::new(Vec::new())),
            output: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Write for MarlinMockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut input = self.input.lock().unwrap();
        input.extend_from_slice(buf);

        if let Some(pos) = input.iter().position(|&b| b == b'\n' || b == b'\r') {
            let line_bytes = input.drain(..=pos).collect::<Vec<u8>>();
            let line = String::from_utf8_lossy(&line_bytes).trim().to_string();

            let response = if line.starts_with("M105") {
                "ok T:210.0 /210.0 B:60.0 /60.0\n".to_string()
            } else if line.starts_with("M28") {
                "Writing to file: mock.gcode\nok\n".to_string()
            } else if line.starts_with("M29") {
                "Done saving file\nok\n".to_string()
            } else if line.starts_with("M112") {
                "ok Emergency Stop\n".to_string()
            } else {
                "ok\n".to_string()
            };

            let mut output = self.output.lock().unwrap();
            output.extend_from_slice(response.as_bytes());
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for MarlinMockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut output = self.output.lock().unwrap();
        if output.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "No data in mock stream",
            ));
        }
        let len = std::cmp::min(buf.len(), output.len());
        buf[..len].copy_from_slice(&output[..len]);
        output.drain(..len);
        Ok(len)
    }
}

enum MarlinStream {
    Physical(Box<dyn serialport::SerialPort>),
    Mock(MarlinMockStream),
}

impl Read for MarlinStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            MarlinStream::Physical(ref mut p) => p.read(buf),
            MarlinStream::Mock(ref mut m) => m.read(buf),
        }
    }
}

impl Write for MarlinStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            MarlinStream::Physical(ref mut p) => p.write(buf),
            MarlinStream::Mock(ref mut m) => m.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            MarlinStream::Physical(ref mut p) => p.flush(),
            MarlinStream::Mock(ref mut m) => m.flush(),
        }
    }
}

#[allow(dead_code)]
pub struct MarlinSerialAdapter {
    profile: PrinterProfile,
    config: PrinterConnectionConfig,
    port: Arc<Mutex<Option<MarlinStream>>>,
}

impl MarlinSerialAdapter {
    pub fn new(profile: PrinterProfile, config: PrinterConnectionConfig) -> Self {
        Self {
            profile,
            config,
            port: Arc::new(Mutex::new(None)),
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

    fn send_command_sync(stream: &mut MarlinStream, cmd: &str) -> Result<String, AdapterError> {
        stream
            .write_all(cmd.as_bytes())
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;
        stream
            .flush()
            .map_err(|e| AdapterError::CommandFailed(e.to_string()))?;

        let mut response = String::new();
        let mut temp_buf = [0; 1024];
        let start_time = std::time::Instant::now();

        loop {
            match stream.read(&mut temp_buf) {
                Ok(n) => {
                    response.push_str(&String::from_utf8_lossy(&temp_buf[..n]));
                    if response.contains("ok") {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start_time.elapsed() > std::time::Duration::from_secs(2) {
                        return Err(AdapterError::CommandFailed(
                            "Timeout waiting for ok".to_string(),
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(e) => return Err(AdapterError::CommandFailed(e.to_string())),
            }
        }
        Ok(response)
    }
}

#[async_trait]
impl PrinterAdapter for MarlinSerialAdapter {
    async fn connect(&mut self) -> Result<(), AdapterError> {
        let port_lock = self.port.clone();
        let config = self.config.clone();

        tokio::task::spawn_blocking(move || {
            let stream = if config.mode == printproof3d_core::connection::ConnectionMode::Simulator
            {
                MarlinStream::Mock(MarlinMockStream::new())
            } else {
                let path = config.serial_path.as_deref().ok_or_else(|| {
                    AdapterError::ConnectionFailed("Serial path is missing".to_string())
                })?;
                let baud = config.serial_baud_rate.unwrap_or(115200);

                let port = serialport::new(path, baud)
                    .timeout(std::time::Duration::from_millis(100))
                    .open()
                    .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;
                MarlinStream::Physical(port)
            };

            let mut guard = port_lock.lock().unwrap();
            *guard = Some(stream);
            Ok(())
        })
        .await
        .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        let mut guard = self.port.lock().unwrap();
        *guard = None;
        Ok(())
    }

    async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
        let port_lock = self.port.clone();

        let resp = tokio::task::spawn_blocking(move || {
            let mut guard = port_lock.lock().unwrap();
            let stream = guard
                .as_mut()
                .ok_or_else(|| AdapterError::CommandFailed("Not connected".to_string()))?;
            Self::send_command_sync(stream, "M105\n")
        })
        .await
        .map_err(|e| AdapterError::CommandFailed(e.to_string()))??;

        let mut tool_temp = 0.0;
        let mut tool_target = 0.0;
        let mut bed_temp = 0.0;
        let mut bed_target = 0.0;

        if let Some(t_pos) = resp.find("T:") {
            let t_part = &resp[t_pos + 2..];
            if let Some(slash_pos) = t_part.find('/') {
                tool_temp = t_part[..slash_pos].trim().parse().unwrap_or(0.0);
                let remaining = &t_part[slash_pos + 1..];
                let end_pos = remaining
                    .find(|c: char| c.is_whitespace())
                    .unwrap_or(remaining.len());
                tool_target = remaining[..end_pos].trim().parse().unwrap_or(0.0);
            }
        }
        if let Some(b_pos) = resp.find("B:") {
            let b_part = &resp[b_pos + 2..];
            if let Some(slash_pos) = b_part.find('/') {
                bed_temp = b_part[..slash_pos].trim().parse().unwrap_or(0.0);
                let remaining = &b_part[slash_pos + 1..];
                let end_pos = remaining
                    .find(|c: char| c.is_whitespace())
                    .unwrap_or(remaining.len());
                bed_target = remaining[..end_pos].trim().parse().unwrap_or(0.0);
            }
        }

        Ok(PrinterTelemetry {
            state: PrinterState::Idle,
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
        let content = tokio::fs::read_to_string(local_path)
            .await
            .map_err(|e| AdapterError::UploadFailed(e.to_string()))?;

        let port_lock = self.port.clone();
        let remote_name_str = remote_name.to_string();
        tokio::task::spawn_blocking(move || {
            let mut guard = port_lock.lock().unwrap();
            let stream = guard
                .as_mut()
                .ok_or_else(|| AdapterError::UploadFailed("Not connected".to_string()))?;

            Self::send_command_sync(stream, &format!("M28 {}\n", remote_name_str))?;
            for line in content.lines() {
                Self::send_command_sync(stream, &format!("{}\n", line))?;
            }
            Self::send_command_sync(stream, "M29\n")?;
            Ok(remote_name_str)
        })
        .await
        .map_err(|e| AdapterError::UploadFailed(e.to_string()))?
    }

    async fn start_job(&self, _file_id: &str) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let port_lock = self.port.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = port_lock.lock().unwrap();
            let stream = guard
                .as_mut()
                .ok_or_else(|| AdapterError::CommandFailed("Not connected".to_string()))?;
            Self::send_command_sync(stream, "M24\n")?;
            Ok(())
        })
        .await
        .map_err(|e| AdapterError::CommandFailed(e.to_string()))?
    }

    async fn pause_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let port_lock = self.port.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = port_lock.lock().unwrap();
            let stream = guard
                .as_mut()
                .ok_or_else(|| AdapterError::CommandFailed("Not connected".to_string()))?;
            Self::send_command_sync(stream, "M25\n")?;
            Ok(())
        })
        .await
        .map_err(|e| AdapterError::CommandFailed(e.to_string()))?
    }

    async fn resume_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let port_lock = self.port.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = port_lock.lock().unwrap();
            let stream = guard
                .as_mut()
                .ok_or_else(|| AdapterError::CommandFailed("Not connected".to_string()))?;
            Self::send_command_sync(stream, "M24\n")?;
            Ok(())
        })
        .await
        .map_err(|e| AdapterError::CommandFailed(e.to_string()))?
    }

    async fn cancel_job(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let port_lock = self.port.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = port_lock.lock().unwrap();
            let stream = guard
                .as_mut()
                .ok_or_else(|| AdapterError::CommandFailed("Not connected".to_string()))?;
            Self::send_command_sync(stream, "M410\n")?;
            Ok(())
        })
        .await
        .map_err(|e| AdapterError::CommandFailed(e.to_string()))?
    }

    async fn emergency_stop(&self) -> Result<(), AdapterError> {
        self.check_dispatch_control()?;
        let port_lock = self.port.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = port_lock.lock().unwrap();
            let stream = guard
                .as_mut()
                .ok_or_else(|| AdapterError::CommandFailed("Not connected".to_string()))?;
            Self::send_command_sync(stream, "M112\n")?;
            Ok(())
        })
        .await
        .map_err(|e| AdapterError::CommandFailed(e.to_string()))?
    }
}

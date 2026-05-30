// PrintProof3D Developer SDK
use printproof3d_adapters::PrinterAdapter;


pub mod mocks;

pub fn sdk_init() -> &'static str {
    "initialized"
}

/// Automated conformance test suite for PrinterAdapter implementations.
pub async fn run_conformance_tests<A: PrinterAdapter>(adapter: &mut A) -> Result<(), String> {
    // 1. Connect
    adapter.connect().await.map_err(|e| format!("conformance failure on connect: {:?}", e))?;

    // 2. Status & Telemetry
    let telemetry = adapter.get_status().await.map_err(|e| format!("conformance failure on get_status: {:?}", e))?;
    if telemetry.state.is_empty() {
        return Err("conformance failure: state is empty".to_string());
    }

    // 3. Pause, Resume, and Cancel
    adapter.pause_job().await.map_err(|e| format!("conformance failure on pause_job: {:?}", e))?;
    adapter.resume_job().await.map_err(|e| format!("conformance failure on resume_job: {:?}", e))?;
    adapter.cancel_job().await.map_err(|e| format!("conformance failure on cancel_job: {:?}", e))?;

    // 4. Disconnect
    adapter.disconnect().await.map_err(|e| format!("conformance failure on disconnect: {:?}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::mocks::*;
    use async_trait::async_trait;
    use printproof3d_adapters::{AdapterError, PrinterTelemetry};

    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::Path;

    // Helper client to test the conformance suite against RrfMockServer
    struct RrfTestClient {
        port: u16,
    }

    #[async_trait]
    impl PrinterAdapter for RrfTestClient {
        async fn connect(&mut self) -> Result<(), AdapterError> {
            let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.port))
                .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;
            stream.write_all(b"GET /rr_connect HTTP/1.1\r\n\r\n").unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            if response.contains("200 OK") {
                Ok(())
            } else {
                Err(AdapterError::ConnectionFailed("RRF connect failed".to_string()))
            }
        }

        async fn disconnect(&mut self) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
            let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.port))
                .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;
            stream.write_all(b"GET /rr_status HTTP/1.1\r\n\r\n").unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            if response.contains("200 OK") {
                // Find start of JSON body
                if let Some(pos) = response.find("\r\n\r\n") {
                    let body = &response[pos+4..];
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                        let state = json.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        return Ok(PrinterTelemetry {
                            state,
                            tool_temp: 210.0,
                            tool_target: 210.0,
                            bed_temp: 60.0,
                            bed_target: 60.0,
                            progress: 50.0,
                            current_file: None,
                        });
                    }
                }
            }
            Err(AdapterError::CommandFailed("Status failed".to_string()))
        }

        async fn upload_file(&self, _local_path: &Path, _remote_name: &str) -> Result<String, AdapterError> {
            Ok("success".to_string())
        }

        async fn start_job(&self, _file_id: &str) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn pause_job(&self) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn resume_job(&self) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn cancel_job(&self) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn emergency_stop(&self) -> Result<(), AdapterError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_sdk_conformance_rrf() {
        let server = RrfMockServer::start();
        let mut client = RrfTestClient { port: server.port };
        
        let res = run_conformance_tests(&mut client).await;
        assert!(res.is_ok(), "Conformance run failed: {:?}", res);
        
        server.stop();
    }

    struct BambuTestClient {
        mqtt_port: u16,
        ftp_port: u16,
    }

    #[async_trait]
    impl PrinterAdapter for BambuTestClient {
        async fn connect(&mut self) -> Result<(), AdapterError> {
            let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.mqtt_port))
                .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;
            let connect_pkt = [0x10, 0x0c, 0x00, 0x04, b'M', b'Q', b'T', b'T', 0x04, 0x02, 0x00, 0x3c, 0x00, 0x00];
            stream.write_all(&connect_pkt).unwrap();
            let mut connack = [0; 4];
            stream.read_exact(&mut connack).unwrap();
            if connack == [0x20, 0x02, 0x00, 0x00] {
                let subscribe_pkt = [0x82, 0x09, 0x00, 0x01, 0x00, 0x04, b't', b'e', b's', b't', 0x00];
                stream.write_all(&subscribe_pkt).unwrap();
                let mut suback = [0; 5];
                stream.read_exact(&mut suback).unwrap();
                Ok(())
            } else {
                Err(AdapterError::ConnectionFailed("Bambu MQTT connect failed".to_string()))
            }
        }

        async fn disconnect(&mut self) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn get_status(&self) -> Result<PrinterTelemetry, AdapterError> {
            let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.mqtt_port))
                .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;
            let connect_pkt = [0x10, 0x0c, 0x00, 0x04, b'M', b'Q', b'T', b'T', 0x04, 0x02, 0x00, 0x3c, 0x00, 0x00];
            stream.write_all(&connect_pkt).unwrap();
            let mut connack = [0; 4];
            stream.read_exact(&mut connack).unwrap();
            let subscribe_pkt = [0x82, 0x09, 0x00, 0x01, 0x00, 0x04, b't', b'e', b's', b't', 0x00];
            stream.write_all(&subscribe_pkt).unwrap();
            let mut suback = [0; 5];
            stream.read_exact(&mut suback).unwrap();

            let mut telemetry_header = [0; 2];
            stream.read_exact(&mut telemetry_header).unwrap();
            let rem_len = telemetry_header[1] as usize;
            let mut payload = vec![0; rem_len];
            stream.read_exact(&mut payload).unwrap();
            let telemetry_str = String::from_utf8_lossy(&payload);
            if telemetry_str.contains("gcode_state") {
                Ok(PrinterTelemetry {
                    state: "IDLE".to_string(),
                    tool_temp: 21.0,
                    tool_target: 21.0,
                    bed_temp: 18.0,
                    bed_target: 18.0,
                    progress: 0.0,
                    current_file: None,
                })
            } else {
                Err(AdapterError::CommandFailed("Invalid telemetry payload".to_string()))
            }
        }

        async fn upload_file(&self, _local_path: &Path, _remote_name: &str) -> Result<String, AdapterError> {
            let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.ftp_port))
                .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?;
            let mut buffer = [0; 64];
            let n = stream.read(&mut buffer).unwrap();
            let greeting = String::from_utf8_lossy(&buffer[..n]);
            if greeting.contains("220 Mock FTP ready") {
                Ok("uploaded_success".to_string())
            } else {
                Err(AdapterError::UploadFailed("FTP greeting failed".to_string()))
            }
        }

        async fn start_job(&self, _file_id: &str) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn pause_job(&self) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn resume_job(&self) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn cancel_job(&self) -> Result<(), AdapterError> {
            Ok(())
        }

        async fn emergency_stop(&self) -> Result<(), AdapterError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_sdk_conformance_bambu() {
        let mqtt_server = BambuMqttMock::start();
        let ftp_server = BambuFtpMock::start();
        let mut client = BambuTestClient {
            mqtt_port: mqtt_server.port,
            ftp_port: ftp_server.port,
        };

        let res = run_conformance_tests(&mut client).await;
        assert!(res.is_ok(), "Bambu Conformance run failed: {:?}", res);

        mqtt_server.stop();
        ftp_server.stop();
    }


    #[test]
    fn test_rrf_mock() {
        let server = RrfMockServer::start();
        
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", server.port)).unwrap();
        stream.write_all(b"GET /rr_status HTTP/1.1\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("heads"));
        
        server.stop();
    }

    #[test]
    fn test_bambu_ftp_mock() {
        let server = BambuFtpMock::start();
        
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", server.port)).unwrap();
        let mut buffer = [0; 64];
        let n = stream.read(&mut buffer).unwrap();
        let greeting = String::from_utf8_lossy(&buffer[..n]);
        assert!(greeting.contains("220 Mock FTP ready"));
        
        server.stop();
    }

    #[test]
    fn test_bambu_mqtt_mock() {
        let server = BambuMqttMock::start();
        
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", server.port)).unwrap();
        
        // Send Connect Packet
        let connect_pkt = [0x10, 0x0c, 0x00, 0x04, b'M', b'Q', b'T', b'T', 0x04, 0x02, 0x00, 0x3c, 0x00, 0x00];
        stream.write_all(&connect_pkt).unwrap();
        
        let mut connack = [0; 4];
        stream.read_exact(&mut connack).unwrap();
        assert_eq!(connack, [0x20, 0x02, 0x00, 0x00]);

        // Send Subscribe Packet (Packet ID: 1, Topic: "test")
        let subscribe_pkt = [0x82, 0x09, 0x00, 0x01, 0x00, 0x04, b't', b'e', b's', b't', 0x00];
        stream.write_all(&subscribe_pkt).unwrap();
        
        let mut suback = [0; 5];
        stream.read_exact(&mut suback).unwrap();
        assert_eq!(suback[0], 0x90);
        assert_eq!(suback[1], 0x03);
        assert_eq!(suback[2], 0x00);
        assert_eq!(suback[3], 0x01);
        assert_eq!(suback[4], 0x00);

        // Telemetry loop will write telemetry messages. Let's read one telemetry message.
        // Telemetry MQTT packet should start with 0x30 (Publish)
        let mut telemetry_header = [0; 2];
        stream.read_exact(&mut telemetry_header).unwrap();
        assert_eq!(telemetry_header[0], 0x30); // Publish type
        
        let rem_len = telemetry_header[1] as usize;
        let mut payload = vec![0; rem_len];
        stream.read_exact(&mut payload).unwrap();
        
        let telemetry_str = String::from_utf8_lossy(&payload);
        assert!(telemetry_str.contains("gcode_state"));
        assert!(telemetry_str.contains("IDLE"));
        
        server.stop();
    }
}

// PrintProof3D Developer SDK
use printproof3d_adapters::PrinterAdapter;

pub mod mocks;

pub fn sdk_init() {}

/// Automated conformance test suite for PrinterAdapter implementations.
pub async fn run_conformance_tests<A: PrinterAdapter>(adapter: &mut A) -> Result<(), String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CONFORMANCE_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let file_id = CONFORMANCE_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let filename = format!("test_upload_conformance_{}.gcode", file_id);

    // 1. Connect
    adapter
        .connect()
        .await
        .map_err(|e| format!("conformance failure on connect: {:?}", e))?;

    // 2. Status & Telemetry
    let _telemetry = adapter
        .get_status()
        .await
        .map_err(|e| format!("conformance failure on get_status: {:?}", e))?;

    // 3. Upload File
    let temp_file = std::env::current_dir().unwrap().join(&filename);
    std::fs::write(&temp_file, b"; dummy gcode")
        .map_err(|e| format!("failed to write conformance temp file: {}", e))?;
    adapter
        .upload_file(&temp_file, &filename)
        .await
        .map_err(|e| {
            let _ = std::fs::remove_file(&temp_file);
            format!("conformance failure on upload_file: {:?}", e)
        })?;
    let _ = std::fs::remove_file(&temp_file);

    // 4. Start Job
    adapter
        .start_job(&filename)
        .await
        .map_err(|e| format!("conformance failure on start_job: {:?}", e))?;

    // 5. Pause, Resume, and Cancel
    adapter
        .pause_job()
        .await
        .map_err(|e| format!("conformance failure on pause_job: {:?}", e))?;
    adapter
        .resume_job()
        .await
        .map_err(|e| format!("conformance failure on resume_job: {:?}", e))?;
    adapter
        .cancel_job()
        .await
        .map_err(|e| format!("conformance failure on cancel_job: {:?}", e))?;

    // 6. Emergency Stop
    adapter
        .emergency_stop()
        .await
        .map_err(|e| format!("conformance failure on emergency_stop: {:?}", e))?;

    // 7. Disconnect
    adapter
        .disconnect()
        .await
        .map_err(|e| format!("conformance failure on disconnect: {:?}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::mocks::*;
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    use printproof3d_adapters::bambu::BambuAdapter;
    use printproof3d_adapters::moonraker::MoonrakerAdapter;
    use printproof3d_adapters::octoprint::OctoPrintAdapter;
    use printproof3d_adapters::prusalink::PrusaLinkAdapter;
    use printproof3d_adapters::rrf::RrfAdapter;
    use printproof3d_adapters::serial::MarlinSerialAdapter;
    use printproof3d_core::{
        connection::{AuthType, ConnectionMode, DispatchPolicy, PrinterConnectionConfig},
        BedShape, BuildVolume, FirmwareFlavor, PrinterProfile, ProtocolFamily,
    };

    fn dummy_profile(protocol: ProtocolFamily) -> PrinterProfile {
        PrinterProfile {
            manufacturer: "Prusa".to_string(),
            model: "MK4".to_string(),
            protocol_family: protocol,
            build_volume: BuildVolume::Rectangular {
                x: 250.0,
                y: 210.0,
                z: 220.0,
            },
            bed_shape: BedShape::Rectangular,
            nozzle_diameters: vec![0.4],
            default_nozzle_diameter: 0.4,
            min_layer_height: 0.05,
            max_layer_height: 0.30,
            max_hotend_temp: 300.0,
            max_bed_temp: 120.0,
            has_enclosure: false,
            supports_mmu: false,
            firmware_flavor: FirmwareFlavor::Prusa,
            supported_file_types: vec!["gcode".to_string()],
            supports_direct_upload: true,
            supports_pause_resume: true,
            supports_cancel: true,
            supports_job_progress: true,
            supports_webcam: false,
            supports_chamber_temp: false,
            known_quirks: vec![],
            unsafe_commands: vec![],
            filename_restrictions: None,
        }
    }

    #[tokio::test]
    async fn test_sdk_conformance_rrf() {
        let server = RrfMockServer::start();
        let profile = dummy_profile(ProtocolFamily::RepRapFirmware);
        let config = PrinterConnectionConfig {
            name: "RRF Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::RepRapFirmware,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = RrfAdapter::new(profile, config);

        let res = run_conformance_tests(&mut adapter).await;
        assert!(res.is_ok(), "RRF Conformance run failed: {:?}", res);

        server.stop();
    }

    #[tokio::test]
    async fn test_sdk_conformance_octoprint() {
        let mut server = OctoPrintMockServer::start();
        let profile = dummy_profile(ProtocolFamily::OctoPrint);
        let config = PrinterConnectionConfig {
            name: "OctoPrint Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::OctoPrint,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = OctoPrintAdapter::new(profile, config);

        let res = run_conformance_tests(&mut adapter).await;
        assert!(res.is_ok(), "OctoPrint Conformance run failed: {:?}", res);

        server.stop();
    }

    #[tokio::test]
    async fn test_sdk_conformance_moonraker() {
        let mut server = MoonrakerMockServer::start();
        let profile = dummy_profile(ProtocolFamily::Klipper);
        let config = PrinterConnectionConfig {
            name: "Moonraker Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::Klipper,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = MoonrakerAdapter::new(profile, config);

        let res = run_conformance_tests(&mut adapter).await;
        assert!(res.is_ok(), "Moonraker Conformance run failed: {:?}", res);

        server.stop();
    }

    #[tokio::test]
    async fn test_sdk_conformance_bambu() {
        let mqtt_server = BambuMqttMock::start();
        let ftp_server = BambuFtpMock::start();
        let profile = dummy_profile(ProtocolFamily::BambuMqtt);
        let config = PrinterConnectionConfig {
            name: "Bambu Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::BambuMqtt,
            base_url: Some(format!(
                "127.0.0.1:{}:{}",
                mqtt_server.port, ftp_server.port
            )),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = BambuAdapter::new(profile, config);

        let res = run_conformance_tests(&mut adapter).await;
        assert!(res.is_ok(), "Bambu Conformance run failed: {:?}", res);

        mqtt_server.stop();
        ftp_server.stop();
    }

    #[tokio::test]
    async fn test_sdk_conformance_serial() {
        let profile = dummy_profile(ProtocolFamily::MarlinSerial);
        let config = PrinterConnectionConfig {
            name: "Marlin Serial Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::MarlinSerial,
            base_url: None,
            serial_path: Some("COM3".to_string()),
            serial_baud_rate: Some(115200),
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = MarlinSerialAdapter::new(profile, config);

        let res = run_conformance_tests(&mut adapter).await;
        assert!(
            res.is_ok(),
            "Marlin Serial Conformance run failed: {:?}",
            res
        );
    }

    #[tokio::test]
    async fn test_sdk_conformance_prusalink() {
        let mut server = PrusaLinkMockServer::start();
        let profile = dummy_profile(ProtocolFamily::PrusaLink);
        let config = PrinterConnectionConfig {
            name: "PrusaLink Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::PrusaLink,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::Digest,
            api_key_env_var: None,
            username: Some("maker".to_string()),
            password_env_var: Some("PRUSALINK_PASSWORD".to_string()),
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        std::env::set_var("PRUSALINK_PASSWORD", "makerpass");
        let mut adapter = PrusaLinkAdapter::new(profile, config);

        let res = run_conformance_tests(&mut adapter).await;
        assert!(res.is_ok(), "PrusaLink Conformance run failed: {:?}", res);

        server.stop();
    }

    #[tokio::test]
    async fn test_octoprint_mock() {
        let mut server = OctoPrintMockServer::start();

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", server.port))
            .await
            .unwrap();
        stream
            .write_all(b"GET /api/printer HTTP/1.1\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Operational"));

        server.stop();
    }

    #[tokio::test]
    async fn test_moonraker_mock() {
        let mut server = MoonrakerMockServer::start();

        // Check REST status
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", server.port))
            .await
            .unwrap();
        stream
            .write_all(b"GET /printer/info HTTP/1.1\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("ready"));

        // Check WS connection
        let ws_url = format!("ws://127.0.0.1:{}/websocket", server.port);
        let (mut ws_stream, _) = tokio_tungstenite::connect_async(ws_url).await.unwrap();

        use futures_util::{SinkExt, StreamExt};
        // Expect initial telemetry notification
        let msg = ws_stream.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains("notify_status_update"));

        // Send subscribe
        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                "printer.objects.subscribe".into(),
            ))
            .await
            .unwrap();
        let msg = ws_stream.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains("extruder"));

        ws_stream.close(None).await.unwrap();
        server.stop();
    }

    #[tokio::test]
    async fn test_prusalink_mock() {
        let mut server = PrusaLinkMockServer::start();

        // 1. Send request without auth
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", server.port))
            .await
            .unwrap();
        stream
            .write_all(b"GET /api/v1/status HTTP/1.1\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("401 Unauthorized"));
        assert!(response.to_lowercase().contains("www-authenticate: digest"));

        // 2. Parse challenge and send request with auth
        let auth_line = response
            .lines()
            .find(|l| l.to_lowercase().starts_with("www-authenticate:"))
            .unwrap();
        let challenge = auth_line["www-authenticate:".len()..].trim();

        let mut params = std::collections::HashMap::new();
        let s = challenge.strip_prefix("Digest ").unwrap_or(challenge);
        for item in s.split(',') {
            let mut parts = item.splitn(2, '=');
            if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                let key = k.trim().to_string();
                let value = v.trim().trim_matches('"').to_string();
                params.insert(key, value);
            }
        }

        let realm = params.get("realm").cloned().unwrap_or_default();
        let nonce = params.get("nonce").cloned().unwrap_or_default();
        let qop = params.get("qop").cloned();

        let nc = "00000001";
        let cnonce = "clientnonce";

        let ha1 = format!("{:x}", md5::compute("maker:PrusaLink:makerpass"));
        let ha2 = format!("{:x}", md5::compute("GET:/api/v1/status"));
        let digest_response = format!(
            "{:x}",
            md5::compute(format!(
                "{}:{}:{}:{}:{}:{}",
                ha1,
                nonce,
                nc,
                cnonce,
                qop.as_deref().unwrap(),
                ha2
            ))
        );

        let auth_header = format!(
            "Authorization: Digest username=\"maker\", realm=\"{}\", nonce=\"{}\", uri=\"/api/v1/status\", response=\"{}\", qop=auth, nc={}, cnonce=\"{}\"",
            realm, nonce, digest_response, nc, cnonce
        );

        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", server.port))
            .await
            .unwrap();
        stream
            .write_all(
                format!(
                    "GET /api/v1/status HTTP/1.1\r\nConnection: close\r\n{}\r\n\r\n",
                    auth_header
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("temp-nozzle"));

        server.stop();
    }

    #[test]
    fn test_rrf_mock() {
        let server = RrfMockServer::start();

        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", server.port)).unwrap();
        stream
            .write_all(b"GET /rr_status HTTP/1.1\r\n\r\n")
            .unwrap();
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
        let connect_pkt = [
            0x10, 0x0c, 0x00, 0x04, b'M', b'Q', b'T', b'T', 0x04, 0x02, 0x00, 0x3c, 0x00, 0x00,
        ];
        stream.write_all(&connect_pkt).unwrap();

        let mut connack = [0; 4];
        stream.read_exact(&mut connack).unwrap();
        assert_eq!(connack, [0x20, 0x02, 0x00, 0x00]);

        // Send Subscribe Packet (Packet ID: 1, Topic: "test")
        let subscribe_pkt = [
            0x82, 0x09, 0x00, 0x01, 0x00, 0x04, b't', b'e', b's', b't', 0x00,
        ];
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

    #[tokio::test]
    async fn test_rrf_telemetry_and_errors() {
        let server = RrfMockServer::start();
        let profile = dummy_profile(ProtocolFamily::RepRapFirmware);
        let config = PrinterConnectionConfig {
            name: "RRF Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::RepRapFirmware,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = RrfAdapter::new(profile, config);

        assert!(adapter.connect().await.is_ok());

        let telemetry = adapter.get_status().await.unwrap();
        assert_eq!(telemetry.state, printproof3d_adapters::PrinterState::Idle);
        assert_eq!(telemetry.tool_temp, 210.0);
        assert_eq!(telemetry.tool_target, 210.0);
        assert_eq!(telemetry.bed_temp, 60.0);
        assert_eq!(telemetry.bed_target, 60.0);

        server.stop();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(adapter.connect().await.is_err());
        assert!(adapter.get_status().await.is_err());
        assert!(adapter
            .upload_file(std::path::Path::new("dummy.gcode"), "dummy.gcode")
            .await
            .is_err());
        assert!(adapter.start_job("dummy").await.is_err());
    }

    #[tokio::test]
    async fn test_octoprint_telemetry_and_errors() {
        let mut server = OctoPrintMockServer::start();
        let profile = dummy_profile(ProtocolFamily::OctoPrint);
        let config = PrinterConnectionConfig {
            name: "OctoPrint Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::OctoPrint,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = OctoPrintAdapter::new(profile, config);

        assert!(adapter.connect().await.is_ok());

        let telemetry = adapter.get_status().await.unwrap();
        assert_eq!(telemetry.state, printproof3d_adapters::PrinterState::Idle);
        assert_eq!(telemetry.tool_temp, 210.0);
        assert_eq!(telemetry.tool_target, 210.0);
        assert_eq!(telemetry.bed_temp, 60.0);
        assert_eq!(telemetry.bed_target, 60.0);

        server.stop();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(adapter.connect().await.is_err());
        assert!(adapter.get_status().await.is_err());
        assert!(adapter
            .upload_file(std::path::Path::new("dummy.gcode"), "dummy.gcode")
            .await
            .is_err());
        assert!(adapter.start_job("dummy").await.is_err());
    }

    #[tokio::test]
    async fn test_moonraker_telemetry_and_errors() {
        let mut server = MoonrakerMockServer::start();
        let profile = dummy_profile(ProtocolFamily::Klipper);
        let config = PrinterConnectionConfig {
            name: "Moonraker Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::Klipper,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = MoonrakerAdapter::new(profile, config);

        assert!(adapter.connect().await.is_ok());

        let telemetry = adapter.get_status().await.unwrap();
        assert_eq!(telemetry.state, printproof3d_adapters::PrinterState::Idle);
        assert_eq!(telemetry.tool_temp, 210.0);
        assert_eq!(telemetry.tool_target, 210.0);
        assert_eq!(telemetry.bed_temp, 60.0);
        assert_eq!(telemetry.bed_target, 60.0);

        server.stop();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(adapter.connect().await.is_err());
        assert!(adapter.get_status().await.is_err());
        assert!(adapter
            .upload_file(std::path::Path::new("dummy.gcode"), "dummy.gcode")
            .await
            .is_err());
        assert!(adapter.start_job("dummy").await.is_err());
    }

    #[tokio::test]
    async fn test_bambu_telemetry_and_errors() {
        let mqtt_server = BambuMqttMock::start();
        let ftp_server = BambuFtpMock::start();
        let profile = dummy_profile(ProtocolFamily::BambuMqtt);
        let config = PrinterConnectionConfig {
            name: "Bambu Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::BambuMqtt,
            base_url: Some(format!(
                "127.0.0.1:{}:{}",
                mqtt_server.port, ftp_server.port
            )),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = BambuAdapter::new(profile, config);

        assert!(adapter.connect().await.is_ok());

        // Wait for MQTT broker loop to publish telemetry
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        let telemetry = adapter.get_status().await.unwrap();
        assert_eq!(telemetry.state, printproof3d_adapters::PrinterState::Idle);
        assert_eq!(telemetry.tool_temp, 21.0);
        assert_eq!(telemetry.tool_target, 21.0);
        assert_eq!(telemetry.bed_temp, 18.0);
        assert_eq!(telemetry.bed_target, 18.0);

        ftp_server.stop();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify error propagation on FTP upload
        assert!(adapter
            .upload_file(std::path::Path::new("dummy.gcode"), "dummy.gcode")
            .await
            .is_err());

        mqtt_server.stop();
    }

    #[tokio::test]
    async fn test_serial_telemetry_and_errors() {
        let profile = dummy_profile(ProtocolFamily::MarlinSerial);
        let config = PrinterConnectionConfig {
            name: "Marlin Serial Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::MarlinSerial,
            base_url: None,
            serial_path: Some("COM3".to_string()),
            serial_baud_rate: Some(115200),
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let mut adapter = MarlinSerialAdapter::new(profile, config);

        assert!(adapter.connect().await.is_ok());

        let telemetry = adapter.get_status().await.unwrap();
        assert_eq!(telemetry.state, printproof3d_adapters::PrinterState::Idle);
        assert_eq!(telemetry.tool_temp, 210.0);
        assert_eq!(telemetry.tool_target, 210.0);
        assert_eq!(telemetry.bed_temp, 60.0);
        assert_eq!(telemetry.bed_target, 60.0);

        assert!(adapter.disconnect().await.is_ok());

        // Verify failure propagation when disconnected
        assert!(adapter.get_status().await.is_err());
        assert!(adapter
            .upload_file(std::path::Path::new("dummy.gcode"), "dummy.gcode")
            .await
            .is_err());
        assert!(adapter.start_job("dummy").await.is_err());
    }

    #[tokio::test]
    async fn test_prusalink_telemetry_and_errors() {
        let mut server = PrusaLinkMockServer::start();
        let profile = dummy_profile(ProtocolFamily::PrusaLink);
        let config = PrinterConnectionConfig {
            name: "PrusaLink Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::PrusaLink,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::Digest,
            api_key_env_var: None,
            username: Some("maker".to_string()),
            password_env_var: Some("PRUSALINK_PASSWORD".to_string()),
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        std::env::set_var("PRUSALINK_PASSWORD", "makerpass");
        let mut adapter = PrusaLinkAdapter::new(profile, config);

        assert!(adapter.connect().await.is_ok());

        let telemetry = adapter.get_status().await.unwrap();
        assert_eq!(telemetry.state, printproof3d_adapters::PrinterState::Idle);
        assert_eq!(telemetry.tool_temp, 210.0);
        assert_eq!(telemetry.tool_target, 210.0);
        assert_eq!(telemetry.bed_temp, 60.0);
        assert_eq!(telemetry.bed_target, 60.0);

        server.stop();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify failure propagation
        assert!(adapter.connect().await.is_err());
        assert!(adapter.get_status().await.is_err());
        assert!(adapter
            .upload_file(std::path::Path::new("dummy.gcode"), "dummy.gcode")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_prusalink_auth_failure() {
        let mut server = PrusaLinkMockServer::start();
        let profile = dummy_profile(ProtocolFamily::PrusaLink);
        let config = PrinterConnectionConfig {
            name: "PrusaLink Test".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::PrusaLink,
            base_url: Some(server.get_url()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::Digest,
            api_key_env_var: None,
            username: Some("maker".to_string()),
            password_env_var: Some("PRUSALINK_PASSWORD_WRONG".to_string()),
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        std::env::set_var("PRUSALINK_PASSWORD_WRONG", "makerpass_wrong");
        let mut adapter = PrusaLinkAdapter::new(profile, config);

        // Verification must fail on connect due to invalid auth
        assert!(adapter.connect().await.is_err());
        server.stop();
    }
}

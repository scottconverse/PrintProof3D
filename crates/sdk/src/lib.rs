// PrintProof3D Developer SDK

pub mod mocks;

pub fn sdk_init() -> &'static str {
    "initialized"
}

#[cfg(test)]
mod tests {
    use super::mocks::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

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

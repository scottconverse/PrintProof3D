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
    use std::thread;

    #[test]
    fn test_rrf_mock() {
        let server = RrfMockServer::start(18898);
        thread::sleep(std::time::Duration::from_millis(150));
        
        let mut stream = TcpStream::connect("127.0.0.1:18898").unwrap();
        stream.write_all(b"GET /rr_status HTTP/1.1\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("heads"));
        
        server.stop();
    }

    #[test]
    fn test_bambu_ftp_mock() {
        let server = BambuFtpMock::start(18899);
        thread::sleep(std::time::Duration::from_millis(150));
        
        let mut stream = TcpStream::connect("127.0.0.1:18899").unwrap();
        let mut buffer = [0; 64];
        let n = stream.read(&mut buffer).unwrap();
        let greeting = String::from_utf8_lossy(&buffer[..n]);
        assert!(greeting.contains("220 Mock FTP ready"));
        
        server.stop();
    }
}

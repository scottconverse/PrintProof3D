use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct RrfMockServer {
    running: Arc<AtomicBool>,
    pub port: u16,
}

impl RrfMockServer {
    pub fn start() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();

        thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
                        .unwrap();
                    let mut buffer = [0; 1024];
                    if let Ok(bytes_read) = stream.read(&mut buffer) {
                        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                        let (response_headers, response_body) = if request.contains("/rr_status") {
                            (
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n",
                                r#"{"status": "I", "coords": {"xyz": [0.0, 0.0, 0.0]}, "temps": {"heads": [210.0], "bed": 60.0}}"#,
                            )
                        } else if request.contains("/rr_connect") {
                            (
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n",
                                r#"{"err": 0, "session": 12345}"#,
                            )
                        } else if request.contains("/rr_upload") {
                            (
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n",
                                r#"{"err": 0}"#,
                            )
                        } else {
                            (
                                "HTTP/1.1 404 NOT FOUND\r\nContent-Type: text/plain\r\n\r\n",
                                "Not Found",
                            )
                        };

                        let response = format!("{}{}", response_headers, response_body);
                        let _ = stream.write_all(response.as_bytes());
                    }
                }
                thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        RrfMockServer { running, port }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn get_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

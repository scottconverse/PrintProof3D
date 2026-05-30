use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct BambuFtpMock {
    running: Arc<AtomicBool>,
    pub port: u16,
}

impl BambuFtpMock {
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
                    let _ = stream.write_all(b"220 Mock FTP ready\r\n");
                    let mut buffer = [0; 512];
                    while let Ok(n) = stream.read(&mut buffer) {
                        if n == 0 { break; }
                        let cmd = String::from_utf8_lossy(&buffer[..n]);
                        let response = if cmd.starts_with("USER") {
                            "331 Password required\r\n"
                        } else if cmd.starts_with("PASS") {
                            "230 Logged in\r\n"
                        } else if cmd.starts_with("SYST") {
                            "215 UNIX Type: L8\r\n"
                        } else if cmd.starts_with("PWD") {
                            "257 \"/\" is current directory\r\n"
                        } else if cmd.starts_with("TYPE") {
                            "200 Type set to I\r\n"
                        } else if cmd.starts_with("PASV") {
                            // Start a transient data listener on port 10240
                            thread::spawn(|| {
                                if let Ok(data_listener) = TcpListener::bind("127.0.0.1:10240") {
                                    if let Ok((mut data_stream, _)) = data_listener.accept() {
                                        let mut data_buf = [0; 4096];
                                        while let Ok(bytes) = data_stream.read(&mut data_buf) {
                                            if bytes == 0 { break; }
                                        }
                                    }
                                }
                            });
                            "227 Entering Passive Mode (127,0,0,1,40,0)\r\n"
                        } else if cmd.starts_with("STOR") {
                            let _ = stream.write_all(b"150 File status okay; about to open data connection.\r\n");
                            thread::sleep(std::time::Duration::from_millis(100));
                            "226 Closing data connection.\r\n"
                        } else if cmd.starts_with("QUIT") {
                            let _ = stream.write_all(b"221 Goodbye\r\n");
                            break;
                        } else {
                            "502 Command not implemented\r\n"
                        };
                        let _ = stream.write_all(response.as_bytes());
                    }
                }
                thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        BambuFtpMock { running, port }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }
}

pub struct BambuMqttMock {
    running: Arc<AtomicBool>,
    pub port: u16,
}

impl BambuMqttMock {
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
                    let mut stream_write = match stream.try_clone() {
                        Ok(s) => s,
                        Err(_) => break,
                    };
                    
                    let mut buffer = [0; 1024];
                    let mut telemetry_spawned = false;
                    
                    #[allow(clippy::while_let_loop)]
                    loop {
                        let n = match stream.read(&mut buffer) {
                            Ok(bytes) => bytes,
                            Err(_) => break,
                        };
                        if n == 0 { break; }
                        
                        let packet_type = buffer[0] >> 4;
                        match packet_type {
                            1 => {
                                let connack = [0x20, 0x02, 0x00, 0x00];
                                if stream_write.write_all(&connack).is_err() {
                                    break;
                                }
                            }
                            8 => {
                                if n >= 4 {
                                    let packet_id_msb = buffer[2];
                                    let packet_id_lsb = buffer[3];
                                    let suback = [0x90, 0x03, packet_id_msb, packet_id_lsb, 0x00];
                                    if stream_write.write_all(&suback).is_err() {
                                        break;
                                    }
                                }
                                
                                if !telemetry_spawned {
                                    telemetry_spawned = true;
                                    let mut telemetry_stream = match stream_write.try_clone() {
                                        Ok(s) => s,
                                        Err(_) => break,
                                    };
                                    let running_telemetry = running_clone.clone();
                                    thread::spawn(move || {
                                        while running_telemetry.load(Ordering::Relaxed) {
                                            let telemetry = r#"{"print":{"gcode_state":"IDLE","mc_percent":0,"mc_remaining_time":0,"nozzle_temper":21.0,"bed_temper":18.0}}"#;
                                            let topic = "device/1234567890/report";
                                            
                                            let mut pkt = Vec::new();
                                            pkt.push(0x30);
                                            
                                            let mut payload = Vec::new();
                                            payload.push((topic.len() >> 8) as u8);
                                            payload.push((topic.len() & 0xFF) as u8);
                                            payload.extend_from_slice(topic.as_bytes());
                                            payload.extend_from_slice(telemetry.as_bytes());
                                            
                                            let rem_len = payload.len();
                                            if rem_len < 128 {
                                                pkt.push(rem_len as u8);
                                            } else {
                                                pkt.push((rem_len & 0x7F | 0x80) as u8);
                                                pkt.push((rem_len >> 7) as u8);
                                            }
                                            pkt.extend(payload);
                                            
                                            if telemetry_stream.write_all(&pkt).is_err() {
                                                break;
                                            }
                                            thread::sleep(std::time::Duration::from_secs(1));
                                        }
                                    });
                                }
                            }
                            12 => {
                                let pingresp = [0xD0, 0x00];
                                if stream_write.write_all(&pingresp).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        BambuMqttMock { running, port }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }
}

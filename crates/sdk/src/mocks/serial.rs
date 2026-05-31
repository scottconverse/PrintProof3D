use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct MarlinMockStream {
    input: Arc<Mutex<Vec<u8>>>,
    output: Arc<Mutex<Vec<u8>>>,
}

impl MarlinMockStream {
    pub fn new() -> Self {
        Self {
            input: Arc::new(Mutex::new(Vec::new())),
            output: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl std::io::Write for MarlinMockStream {
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

impl std::io::Read for MarlinMockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut output = self.output.lock().unwrap();
        if output.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "No data available in mock stream",
            ));
        }
        let len = std::cmp::min(buf.len(), output.len());
        buf[..len].copy_from_slice(&output[..len]);
        output.drain(..len);
        Ok(len)
    }
}

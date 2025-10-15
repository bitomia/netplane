use anyhow::Result;
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ReplayEntry {
    pub _timestamp: String,
    pub sdn_addr: String,
    pub data: Vec<u8>,
}

impl ReplayEntry {
    pub fn parse(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            anyhow::bail!("Invalid log line format");
        }

        let timestamp = parts[0].to_string();
        let sdn_addr = parts[1].to_string();
        let data = parts[2];
        let data = hex::decode(data)?;

        Ok(ReplayEntry {
            _timestamp: timestamp,
            sdn_addr,
            data,
        })
    }
}

#[derive(Clone)]
pub struct TrafficLogger {
    file: Arc<Mutex<std::fs::File>>,
}

impl TrafficLogger {
    pub fn new(filename: &str) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename)?;

        Ok(TrafficLogger {
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub async fn log_packet(&self, sdn_addr: &str, data: &[u8]) {
        let timestamp = Utc::now().to_rfc3339();
        let mut file = self.file.lock().await;

        let hex_data: String = data
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join("");

        let log_entry = format!("{} {} {}\n", timestamp, sdn_addr, hex_data);

        let _ = file.write_all(log_entry.as_bytes());
        let _ = file.flush();
    }
}

use anyhow::{Context, Result};
use log::info;
use netplane_common::transport::{Transport, TransportMode, UdpTransport, WebSocketTransport};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Debug)]
struct TrafficEntry {
    _timestamp: String,
    direction: String,
    addr: String,
    _len: usize,
    data: Vec<u8>,
}

impl TrafficEntry {
    fn parse(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            anyhow::bail!("Invalid log line format");
        }

        let timestamp = parts[0].to_string();
        let direction = parts[1].to_string();
        let addr = parts[2].to_string();

        let len_part = parts[3];
        let len = len_part
            .strip_prefix("len=")
            .context("Missing len= prefix")?
            .parse::<usize>()?;

        let data_part = parts[4];
        let hex_data = data_part
            .strip_prefix("data=")
            .context("Missing data= prefix")?;

        let data = hex::decode(hex_data).context("Failed to decode hex data")?;

        Ok(TrafficEntry {
            _timestamp: timestamp,
            direction,
            addr,
            _len: len,
            data,
        })
    }
}

pub async fn replay_traffic(file_path: &str, transport_mode: TransportMode, delay_seconds: Option<u64>) -> Result<()> {
    info!("Starting traffic replay from: {}", file_path);
    info!("Transport mode: {}", transport_mode.as_string());

    if let Some(delay) = delay_seconds {
        info!("Waiting {} seconds before starting replay...", delay);
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        info!("Starting replay now");
    }

    let file = File::open(file_path).context("Failed to open replay file")?;
    let reader = BufReader::new(file);

    let server_addr = std::env::var("SERVER").unwrap_or("127.0.0.1:5000".to_string());

    match transport_mode {
        TransportMode::UDP => {
            let mut transport = UdpTransport::bind("0.0.0.0:0")
                .await
                .context("Failed to bind UDP socket")?;

            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }

                match TrafficEntry::parse(&line) {
                    Ok(entry) => {
                        if entry.direction == "OUT" {
                            // Skip outbound packets during replay
                            continue;
                        }

                        info!(
                            "Replaying {} bytes to {} (originally from {})",
                            entry.data.len(),
                            server_addr,
                            entry.addr
                        );

                        let addr = SocketAddr::from_str(&server_addr)?;
                        match transport.send(&entry.data, Some(&addr)).await {
                            Ok(_) => {
                                info!("Sent {} bytes", entry.data.len());
                            }
                            Err(e) => {
                                log::error!("Failed to send packet: {}", e);
                            }
                        }

                        // Small delay between packets
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                    Err(e) => {
                        log::warn!("Failed to parse line: {} - {}", line, e);
                    }
                }
            }
        }
        TransportMode::WebSocket => {
            let ws_url = format!("ws://{}", server_addr);
            let mut transport = WebSocketTransport::connect(&ws_url)
                .await
                .context("Failed to connect to WebSocket server")?;

            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }

                match TrafficEntry::parse(&line) {
                    Ok(entry) => {
                        if entry.direction == "OUT" {
                            // Skip outbound packets during replay
                            continue;
                        }

                        info!(
                            "Replaying {} bytes to {} (originally from {})",
                            entry.data.len(),
                            server_addr,
                            entry.addr
                        );

                        match transport.send(&entry.data, None).await {
                            Ok(_) => {
                                info!("Sent {} bytes", entry.data.len());
                            }
                            Err(e) => {
                                log::error!("Failed to send packet: {}", e);
                            }
                        }

                        // Small delay between packets
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                    Err(e) => {
                        log::warn!("Failed to parse line: {} - {}", line, e);
                    }
                }
            }
        }
    }

    info!("Replay completed");
    Ok(())
}

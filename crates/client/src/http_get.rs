use anyhow::{Result, anyhow};
use axum::http::StatusCode;
use std::io::{Read, Write};
use std::net::TcpStream;

const HTTP_PROTO: &str = "http://";
const HTTPS_PROTO: &str = "https://";

fn parse_url(url: String) -> Result<(String, String)> {
    let is_https = match url.find(HTTPS_PROTO) {
        Some(p) => p == 0,
        _ => false,
    };
    let is_http = match url.find(HTTP_PROTO) {
        Some(p) => p == 0,
        _ => false,
    };
    if !is_http && !is_https {
        return Err(anyhow!("Invalid URL. Proto not found."));
    }

    let url_path = if is_http {
        url.split_at(HTTP_PROTO.len()).1
    } else {
        url.split_at(HTTPS_PROTO.len()).1
    };
    let url_path = match url_path.split_once("/") {
        Some(p) => p,
        _ => return Err(anyhow!("Invalid URL. Cannot split path")),
    };
    let host = url_path.0;
    let path = format!("/{}", url_path.1);

    Ok((host.to_string(), path.to_string()))
}

#[derive(Debug)]
pub struct Response {
    pub status_code: StatusCode,
    pub payload: String,
}

pub fn http_get(url: &str, key: &str) -> Result<Response> {
    let (host, path) = parse_url(url.to_string())?;

    let mut stream = TcpStream::connect(host.clone())?;
    let request = format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Authorization: Bearer {}\r\n\
         Connection: close\r\n\
         \r\n\
        ",
        path,
        host.clone(),
        key,
    );

    println!("{request}");

    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    let mut lines = response.lines();
    let http_status = lines.next().ok_or_else(|| anyhow!("Invalid status code"))?;
    let mut http_status = http_status.split(" ");
    http_status
        .next()
        .ok_or_else(|| anyhow!("Invalid status code"))?;
    let http_status_code = http_status
        .next()
        .ok_or_else(|| anyhow!("Invalid status code"))?;
    let status_code = StatusCode::from_bytes(http_status_code.as_bytes())?;

    while let Some(line) = lines.next() {
        if line == "" {
            break;
        }
    }
    let payload = lines.next().unwrap_or("").to_string();
    Ok(Response {
        status_code,
        payload,
    })
}
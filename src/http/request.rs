use std::collections::HashMap;
use std::io::{Read, BufRead, BufReader};
use crate::errors::ServerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    HEAD,
    POST,
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn parse<R: Read>(reader: &mut R) -> Result<Self, ServerError> {
        let mut reader = BufReader::new(reader);

        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        let request_line = request_line.trim();

        if request_line.is_empty() {
            return Err(ServerError::BadRequest("Empty request line".into()));
        }

        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(ServerError::BadRequest(format!(
                "Malformed request line: '{}'", request_line
            )));
        }

        let method = match parts[0] {
            "GET" => HttpMethod::GET,
            "HEAD" => HttpMethod::HEAD,
            "POST" => HttpMethod::POST,
            other => HttpMethod::Unsupported(other.to_string()),
        };

        let path = parts[1].to_string();
        let version = parts[2].to_string();

        if version != "HTTP/1.0" {
            return Err(ServerError::BadRequest(format!(
                "Only HTTP/1.0 is supported (got '{}')", version
            )));
        }

        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                break; // EOF
            }

            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break; // End of headers
            }

            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_string(), value.trim().to_string());
            } else {
                return Err(ServerError::BadRequest(format!(
                    "Invalid header format: '{}'", line
                )));
            }
        }

        let mut body = Vec::new();
        if let Some(content_length) = headers
            .get("Content-Length")
            .and_then(|v| v.parse::<usize>().ok())
        {
            let mut limited = reader.take(content_length as u64);
            limited.read_to_end(&mut body)?;
        }

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
        })
    }
}

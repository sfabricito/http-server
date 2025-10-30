use std::collections::HashMap;
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
pub struct Status {
    pub code: u16,
    pub reason: &'static str,
}

pub const OK: Status = Status { code: 200, reason: "OK" };
pub const ACCEPTED: Status = Status { code: 202, reason: "Accepted" };
pub const BAD_REQUEST: Status = Status { code: 400, reason: "Bad Request" };
pub const NOT_FOUND: Status = Status { code: 404, reason: "Not Found" };
pub const CONFLICT: Status = Status { code: 409, reason: "Conflict" };
pub const TOO_MANY_REQUESTS: Status = Status { code: 429, reason: "Too Many Requests" };
pub const INTERNAL_SERVER_ERROR: Status = Status { code: 500, reason: "Internal Server Error" };
pub const SERVICE_UNAVAILABLE: Status = Status { code: 503, reason: "Service Unavailable" };

#[derive(Debug, Clone)]
pub struct Response {
    pub version: String,
    pub status: Status,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: Status) -> Self {
        Self {
            version: "HTTP/1.0".into(),
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn set_header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }


    fn http_date() -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("Epoch {}", now)
    }

    pub fn to_bytes(&self, is_head: bool) -> Vec<u8> {
        let mut buffer = String::new();

        let _ = write!(
            buffer,
            "{} {} {}\r\n",
            self.version, self.status.code, self.status.reason
        );

        let date = Self::http_date();
        let body_len = self.body.len();

        let _ = writeln!(buffer, "Date: {}", date);
        let _ = writeln!(buffer, "Server: rust-raw/0.1");
        let _ = writeln!(buffer, "Connection: close");
        let _ = writeln!(buffer, "Content-Length: {}", body_len);

        if !self.headers.contains_key("Content-Type") {
            let _ = writeln!(buffer, "Content-Type: text/plain; charset=utf-8");
        }

        for (key, value) in &self.headers {
            let key_lower = key.to_ascii_lowercase();
            if ["content-length", "connection", "date", "server"].contains(&key_lower.as_str()) {
                continue;
            }
            let _ = writeln!(buffer, "{}: {}", key, value);
        }

        buffer.push_str("\r\n");

        let mut response_bytes = buffer.into_bytes();
        if !is_head {
            response_bytes.extend_from_slice(&self.body);
        }

        response_bytes
    }
}

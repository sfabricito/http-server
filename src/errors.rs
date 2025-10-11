use std::fmt::{self, Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum ServerError {
    BadRequest(String),
    NotFound,
    Conflict(String),
    TooManyRequests,
    Internal(String),
    ServiceUnavailable,
    Io(io::Error),
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::BadRequest(msg) => write!(f, "BadRequest: {}", msg),
            ServerError::NotFound => write!(f, "NotFound"),
            ServerError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ServerError::TooManyRequests => write!(f, "TooManyRequests"),
            ServerError::Internal(msg) => write!(f, "Internal: {}", msg),
            ServerError::ServiceUnavailable => write!(f, "ServiceUnavailable"),
            ServerError::Io(e) => write!(f, "IO: {}", e),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<io::Error> for ServerError {
    fn from(value: io::Error) -> Self { ServerError::Io(value) }
}

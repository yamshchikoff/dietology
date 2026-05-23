use std::fmt;
use std::io;

#[derive(Debug)]
pub enum AppError {
    DataFileNotFound(String),
    Io(io::Error),
    JsonParse(serde_json::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataFileNotFound(path) => write!(f, "data file not found: {path}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::JsonParse(e) => write!(f, "JSON parse error: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::JsonParse(e)
    }
}

use serde::Serialize;
use std::fmt;
use std::io;

#[derive(Debug, Serialize)]
pub enum AppError {
    #[serde(rename = "data_file_not_found")]
    DataFileNotFound(String),
    #[serde(rename = "io_error")]
    Io(String),
    #[serde(rename = "json_parse_error")]
    JsonParse(String),
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
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::JsonParse(e.to_string())
    }
}

/// Convenience result alias used throughout the crate
pub type AppResult<T> = Result<T, AppError>;

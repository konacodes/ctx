use serde::Serialize;
use thiserror::Error;

/// Structured error type for machine-readable error output
#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", content = "details")]
pub enum CtxError {
    #[error("Invalid arguments: {message}")]
    #[serde(rename = "invalid_arguments")]
    InvalidArguments { message: String },

    #[error("File not found: {path}")]
    #[serde(rename = "file_not_found")]
    FileNotFound { path: String },

    #[error("Parse error in {file}: {message}")]
    #[serde(rename = "parse_error")]
    ParseError { file: String, message: String },

    #[error("Git error: {message}")]
    #[serde(rename = "git_error")]
    GitError { message: String },

    #[error("IO error: {message}")]
    #[serde(rename = "io_error")]
    IoError { message: String },

    #[error("Serialization error: {message}")]
    #[serde(rename = "serialization_error")]
    SerializationError { message: String },

    #[error("Not a git repository")]
    #[serde(rename = "not_git_repo")]
    NotGitRepo,

    #[error("Timeout after {seconds} seconds")]
    #[serde(rename = "timeout")]
    Timeout { seconds: u64 },
}

/// Exit codes for different error categories
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const USER_ERROR: i32 = 1;      // Invalid arguments, bad input
    pub const RUNTIME_ERROR: i32 = 2;   // File not found, parse error
    pub const GIT_ERROR: i32 = 3;       // Git-related errors
    pub const IO_ERROR: i32 = 4;        // IO/serialization errors
}

impl CtxError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CtxError::InvalidArguments { .. } => exit_codes::USER_ERROR,
            CtxError::FileNotFound { .. } => exit_codes::RUNTIME_ERROR,
            CtxError::ParseError { .. } => exit_codes::RUNTIME_ERROR,
            CtxError::GitError { .. } => exit_codes::GIT_ERROR,
            CtxError::NotGitRepo => exit_codes::GIT_ERROR,
            CtxError::IoError { .. } => exit_codes::IO_ERROR,
            CtxError::SerializationError { .. } => exit_codes::IO_ERROR,
            CtxError::Timeout { .. } => exit_codes::RUNTIME_ERROR,
        }
    }
}

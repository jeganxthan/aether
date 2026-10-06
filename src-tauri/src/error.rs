use std::fmt;

#[derive(Debug)]
pub enum AetherError {
    CommandFailed(String),
    CommandNotFound(String),
    ParseError(String),
    NetworkError(String),
}

impl fmt::Display for AetherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandFailed(message) => write!(f, "Command failed: {message}"),
            Self::CommandNotFound(command) => {
                write!(f, "Command not found: {command}")
            }
            Self::ParseError(message) => write!(f, "Parse error: {message}"),
            Self::NetworkError(message) => write!(f, "Network error: {message}"),
        }
    }
}

impl std::error::Error for AetherError {}

impl From<std::io::Error> for AetherError {
    fn from(error: std::io::Error) -> Self {
        Self::CommandFailed(error.to_string())
    }
}
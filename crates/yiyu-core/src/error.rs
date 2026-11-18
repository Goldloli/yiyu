use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub type YiyuResult<T> = Result<T, YiyuError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Validation,
    Config,
    Permission,
    Capture,
    Authentication,
    RateLimit,
    Timeout,
    Network,
    Protocol,
    EmptyResult,
    Cancelled,
    Storage,
    Update,
    Installer,
    Unsupported,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YiyuError {
    pub kind: ErrorKind,
    pub message: String,
    pub retryable: bool,
    pub field: Option<String>,
    pub actions: Vec<String>,
}

impl YiyuError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable: false,
            field: None,
            actions: Vec::new(),
        }
    }

    pub fn retryable(mut self, value: bool) -> Self {
        self.retryable = value;
        self
    }

    pub fn field(mut self, value: impl Into<String>) -> Self {
        self.field = Some(value.into());
        self
    }

    pub fn action(mut self, value: impl Into<String>) -> Self {
        self.actions.push(value.into());
        self
    }

    pub fn validation(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message).field(field)
    }

    pub fn cancelled() -> Self {
        Self::new(ErrorKind::Cancelled, "translation cancelled")
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Storage, message)
    }
}

impl Display for YiyuError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for YiyuError {}

impl From<std::io::Error> for YiyuError {
    fn from(value: std::io::Error) -> Self {
        Self::storage(value.to_string())
    }
}

impl From<rusqlite::Error> for YiyuError {
    fn from(value: rusqlite::Error) -> Self {
        Self::storage(value.to_string())
    }
}

impl From<serde_json::Error> for YiyuError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(ErrorKind::Protocol, format!("invalid JSON: {value}"))
    }
}

impl From<url::ParseError> for YiyuError {
    fn from(value: url::ParseError) -> Self {
        Self::validation(value.to_string(), "baseUrl")
    }
}

impl From<reqwest::Error> for YiyuError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            return Self::new(ErrorKind::Timeout, value.to_string()).retryable(true);
        }
        if value.is_connect() || value.is_request() {
            return Self::new(ErrorKind::Network, value.to_string()).retryable(true);
        }
        Self::new(ErrorKind::Network, value.to_string()).retryable(true)
    }
}

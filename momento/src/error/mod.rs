use std::fmt;

use colored::Colorize;

pub struct CliError {
    /// Brief, human-readable error message
    pub(crate) msg: String,
    /// Error details for debugging
    detailed_msg: Option<String>,
    /// HTTP status, if this error came from an API response
    http_status: Option<reqwest::StatusCode>,
}

impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {}",
            "ERROR".red().bold(),
            self.detailed_msg.as_ref().unwrap_or(&self.msg)
        )
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", "ERROR".red().bold(), self.msg.red())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(val: serde_json::Error) -> Self {
        CliError::new(format!("{val:?}"))
    }
}

impl CliError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            detailed_msg: None,
            http_status: None,
        }
    }

    pub fn with_details(mut self, detailed_msg: impl Into<String>) -> Self {
        self.detailed_msg = Some(detailed_msg.into());
        self
    }

    pub fn with_optional_details(mut self, detailed_msg: Option<String>) -> Self {
        self.detailed_msg = detailed_msg;
        self
    }

    pub fn details(&self) -> Option<String> {
        self.detailed_msg.clone()
    }

    pub fn with_http_status(mut self, status: reqwest::StatusCode) -> Self {
        self.http_status = Some(status);
        self
    }

    pub fn http_status(&self) -> Option<reqwest::StatusCode> {
        self.http_status
    }
}

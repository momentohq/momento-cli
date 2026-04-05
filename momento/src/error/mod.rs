use std::fmt;

use colored::Colorize;

pub struct CliError {
    /// Brief, human-readable error message
    pub(crate) msg: String,
    /// Error details for debugging
    detailed_msg: Option<String>,
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

impl CliError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            detailed_msg: None,
        }
    }

    pub fn with_details(mut self, detailed_msg: impl Into<String>) -> Self {
        self.detailed_msg = Some(detailed_msg.into());
        self
    }
}

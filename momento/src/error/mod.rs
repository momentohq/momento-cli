use std::fmt;

use colored::Colorize;

pub struct CliError {
    pub(crate) msg: String,
}

impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {:#?}", "ERROR".red().bold(), self.msg.red())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", "ERROR".red().bold(), self.msg.red())
    }
}

impl From<reqwest::Error> for CliError {
    fn from(e: reqwest::Error) -> Self {
        CliError { msg: e.to_string() }
    }
}

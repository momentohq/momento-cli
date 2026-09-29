use super::utils::{ApiKeyInfo, ApiKeyResponse};

use chrono::prelude::DateTime;
use std::fmt;

fn format_epoch_seconds(seconds: u64) -> String {
    match DateTime::from_timestamp(seconds as i64, 0) {
        Some(datetime) => datetime.to_string(),
        None => format!("{seconds} (epoch seconds)"),
    }
}

impl fmt::Display for ApiKeyInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Key ID: {}", self.id)?;
        write!(f, "\nDescription: {}", self.description)?;
        write!(f, "\nRole ID: {}", self.role_id)?;
        write!(
            f,
            "\nExpires At: {}",
            match self.expires_at_epoch_seconds {
                Some(seconds) => format_epoch_seconds(seconds),
                None => "never".to_string(),
            }
        )?;
        write!(
            f,
            "\nIssued At: {}",
            format_epoch_seconds(self.issued_at_epoch_seconds)
        )?;
        Ok(())
    }
}

impl fmt::Display for ApiKeyResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.api_key)?;
        writeln!(f)?;
        write!(f, "\n{}", self.key_info)?;
        Ok(())
    }
}

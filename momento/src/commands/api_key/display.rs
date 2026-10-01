use super::info::{DecodedApiKey, EmbeddedPermissions};
use super::utils::{ApiKeyInfo, ApiKeyResponse};

use chrono::prelude::DateTime;
use std::fmt;

fn format_epoch_seconds(seconds: i64) -> String {
    match DateTime::from_timestamp(seconds, 0) {
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
                Some(seconds) => format_epoch_seconds(seconds as i64),
                None => "never".to_string(),
            }
        )?;
        write!(
            f,
            "\nIssued At: {}",
            format_epoch_seconds(self.issued_at_epoch_seconds as i64)
        )?;
        Ok(())
    }
}

impl fmt::Display for ApiKeyResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.api_key)?;
        if let Some(refresh_token) = &self.refresh_token {
            write!(f, "\n\nRefresh Token:\n\n{refresh_token}")?;
        }
        writeln!(f)?;
        write!(f, "\n{}", self.key_info)?;
        if let Some(previous_key_id) = &self.previous_key_id {
            write!(f, "\nPrevious Key ID: {previous_key_id}")?;
        }
        Ok(())
    }
}

impl fmt::Display for DecodedApiKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Kind: {}", self.kind)?;
        write!(f, "\n{}: {}", self.identity_label, self.identity_value)?;
        if let Some(endpoint) = &self.endpoint {
            write!(f, "\nEndpoint: {endpoint}")?;
        }
        if let Some(expires) = &self.expires {
            write!(
                f,
                "\nExpires At: {}",
                match expires.as_i64() {
                    Some(seconds) => format_epoch_seconds(seconds),
                    None => expires.to_string(),
                }
            )?;
        }
        match &self.permissions {
            EmbeddedPermissions::Absent => {}
            EmbeddedPermissions::Decoded(permissions) => write!(f, "\n{permissions}")?,
            EmbeddedPermissions::Undecodable(error) => {
                write!(f, "\nPermissions: could not be shown: {}", error.msg)?;
                if let Some(details) = error.details() {
                    write!(f, " ({details})")?;
                }
                write!(f, "\n  The raw claim is under \"p\" below.")?;
            }
        }
        write!(
            f,
            "\nClaims: {}",
            serde_json::to_string_pretty(&self.claims).unwrap_or_else(|_| self.claims.to_string())
        )?;
        Ok(())
    }
}

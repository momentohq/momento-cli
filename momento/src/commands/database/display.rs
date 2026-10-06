use super::utils::DatabaseResponse;

use std::fmt;

impl fmt::Display for DatabaseResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Name: {}", self.name)?;
        writeln!(f, "Capacity Pool: {}", self.pool_name)?;
        writeln!(f, "Metrics Config: {}", self.metrics_config)?;
        Ok(())
    }
}

use std::num::IntErrorKind;

use chrono::NaiveDate;

pub const ROLE_PERMISSIONS_SAMPLE: &str = r#"{
  "rules": [
    {
      "type": "account_management",
      "permissions": ["read", "list"]
    },
    {
      "type": "cache",
      "caches": { "name": "prod-cache" },
      "items": "*",
      "permissions": ["read", "write"]
    },
    {
      "type": "function",
      "functions": { "prefix": "webhook-" },
      "caches": { "name": "edge-app" },
      "permissions": ["invoke"]
    }
  ],
  "conditions": [
    {
      "ip_filter": {
        "allowed_cidr_ranges": ["10.0.0.0/8", "192.168.1.0/24", "2001:db8::/32"]
      }
    }
  ]
}"#;

/// Returns a JSON string: `@path` reads from a file, anything else is the JSON itself.
pub fn parse_to_json(s: &str) -> Result<String, String> {
    let json = match s.strip_prefix('@') {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|error| format!("could not read {path}: {error}"))?,
        None => s.to_owned(),
    };
    Ok(json)
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: u32,
    pub max: u32,
}

pub fn parse_bounds(s: &str) -> Result<Bounds, String> {
    let (min, max) = match s.split_once("..") {
        None => (s, s),
        Some(parts) => parts,
    };
    let parse = |bound: &str| {
        bound.trim().parse::<u32>().map_err(|err| match err.kind() {
            IntErrorKind::PosOverflow => format!("'{bound}' is too large (larger than {})", u32::MAX),
            &_ => format!("'{bound}' is not a whole number; expected whole number `N` (pinned) or whole numbers `MIN..MAX`"),
        })
    };
    let bounds = Bounds {
        min: parse(min)?,
        max: parse(max)?,
    };
    if bounds.min > bounds.max {
        return Err(format!(
            "bounds are inverted: {}..{} (MIN must not exceed MAX)",
            bounds.min, bounds.max
        ));
    }
    Ok(bounds)
}

pub fn parse_positive_bounds(s: &str) -> Result<Bounds, String> {
    let bounds = parse_bounds(s)?;
    if bounds.min == 0 {
        return Err("bounds must be >0".to_string());
    }
    Ok(bounds)
}

pub fn parse_positive(s: &str) -> Result<u64, String> {
    let number = s.trim().parse::<u64>().map_err(|err| match err.kind() {
        IntErrorKind::PosOverflow => format!("'{s}' is too large"),
        &_ => format!("'{s}' is not a whole number"),
    })?;
    if number == 0 {
        return Err("must be >0".to_string());
    }
    Ok(number)
}

pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| "Date must be in YYYY-MM-DD format".to_string())
}

pub fn validate_capacity_pool_name(s: &str) -> Result<String, String> {
    if s.is_empty() {
        return Err("pool name cannot be empty".to_string());
    }
    let api_paths = ["families", "instance_types", "metrics"];
    if api_paths.contains(&s) {
        // Gives a nicer error for e.g.:
        // `pool describe -n families` (GET /capacity_pool/families lists the available flex-mode families)
        // `pool create -n families` (405 Method Not Allowed)
        return Err(format!(
            "\"{s}\" is a reserved keyword and cannot be used as a capacity pool name"
        ));
    }
    Ok(s.to_string())
}

/// Accepts and ignores any arguments, so that a moved command parses successfully
/// and can tell the user where it went instead of failing with a parse error.
#[derive(Debug, clap::Args)]
pub struct MovedCommandArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    _ignored: Vec<String>,
}

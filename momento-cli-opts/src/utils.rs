use std::num::IntErrorKind;

use chrono::NaiveDate;

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

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CapacityPoolProvisioningMode {
    Cluster,
    Flex,
}

pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| "Date must be in YYYY-MM-DD format".to_string())
}

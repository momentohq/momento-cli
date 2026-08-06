use chrono::NaiveDate;

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
        bound
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("`{bound}` is not a number; expected `N` (pinned) or `MIN..MAX`"))
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
        return Err("bounds must be at least 1".to_string());
    }
    Ok(bounds)
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CapacityPoolProvisioningMode {
    Explicit,
    Managed,
}

pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| "Date must be in YYYY-MM-DD format".to_string())
}

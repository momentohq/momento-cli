use chrono::NaiveDate;
use regex::Regex;
use std::num::IntErrorKind;

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

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CapacityPoolProvisioningMode {
    Cluster,
    Flex,
}

pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| "Date must be in YYYY-MM-DD format".to_string())
}

fn determine_cell_prefix_for_region(region: &str) -> String {
    match region {
        "us-east-1" | "ap-northeast-1" => "cell",
        "us-west-2" => "cell-4",
        _ => "cell-1",
    }
    .to_string()
}

/// Formats any sample from https://docs.momentohq.com/platform/regions
pub fn determine_endpoint(endpoint_arg: String) -> Result<String, String> {
    let prefixes = ["https://", "api.", "cache."];
    let mut endpoint = endpoint_arg.clone();
    if endpoint_arg.contains(".") {
        for p in prefixes {
            endpoint = endpoint.strip_prefix(p).unwrap_or(&endpoint).to_string();
        }
    } else {
        if !endpoint_arg.starts_with("cell-") {
            let prefix = determine_cell_prefix_for_region(&endpoint_arg);
            endpoint = format!("{prefix}-{endpoint}");
        }
        if let Ok(suffix) = Regex::new(r"-[0-9]-1$") {
            if !suffix.is_match(&endpoint_arg) {
                endpoint += "-1";
            }
        }
        endpoint += ".prod.a.momentohq.com";
    }
    if !endpoint.ends_with(".preprod.a.momentohq.com") {
        if let Ok(structure) =
            Regex::new(r"^cell(-[0-9a-z\-]+)?-[a-z]{2}-[a-z]+-[0-9]+-1\.prod\.a\.momentohq\.com$")
        {
            if !structure.is_match(&endpoint) {
                return Err(
                    "Endpoint structure should match cell[-#]-<aws_region_code>-1.prod.a.momentohq.com. \
                     Please see https://docs.momentohq.com/platform/regions"
                        .to_string(),
                );
            }
        }
    }
    Ok(endpoint)
}

pub fn parse_endpoint(s: &str) -> Result<String, String> {
    let endpoint = determine_endpoint(s.to_string())?;
    if endpoint == s {
        Ok(endpoint)
    } else {
        Err(format!("Do you mean '{endpoint}'?"))
    }
}

#[cfg(test)]
mod tests {
    use super::determine_endpoint;

    #[test]
    fn determine_endpoint_with_valid_endpoint() {
        let endpoint_arg = "cell.preprod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string()).expect("should parse endpoint");
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-us-east-1-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string()).expect("should parse endpoint");
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-4-us-west-2-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string()).expect("should parse endpoint");
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-1-ap-southeast-2-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string()).expect("should parse endpoint");
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-10-us-northeast-3-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string()).expect("should parse endpoint");
        assert_eq!(endpoint_arg, endpoint);

        let endpoint_arg = "cell-foo-bar-us-northeast-3-1.prod.a.momentohq.com";
        let endpoint = determine_endpoint(endpoint_arg.to_string()).expect("should parse endpoint");
        assert_eq!(endpoint_arg, endpoint);
    }

    #[test]
    fn determine_endpoint_with_cell_name() {
        let endpoint =
            determine_endpoint("cell-us-east-1-1".to_string()).expect("should parse endpoint");
        assert_eq!("cell-us-east-1-1.prod.a.momentohq.com", endpoint);

        let endpoint =
            determine_endpoint("cell-4-us-west-2-1".to_string()).expect("should parse endpoint");
        assert_eq!("cell-4-us-west-2-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint("cell-1-ap-southeast-2-1".to_string())
            .expect("should parse endpoint");
        assert_eq!("cell-1-ap-southeast-2-1.prod.a.momentohq.com", endpoint);
    }

    #[test]
    fn determine_endpoint_with_cell_name_no_suffix() {
        let endpoint =
            determine_endpoint("cell-us-east-1".to_string()).expect("should parse endpoint");
        assert_eq!("cell-us-east-1-1.prod.a.momentohq.com", endpoint);

        let endpoint =
            determine_endpoint("cell-4-us-west-2".to_string()).expect("should parse endpoint");
        assert_eq!("cell-4-us-west-2-1.prod.a.momentohq.com", endpoint);

        let endpoint =
            determine_endpoint("cell-1-ap-southeast-2".to_string()).expect("should parse endpoint");
        assert_eq!("cell-1-ap-southeast-2-1.prod.a.momentohq.com", endpoint);
    }

    #[test]
    fn determine_endpoint_with_url() {
        let endpoint = determine_endpoint(
            "https://api.cache.cell-us-east-1-1.prod.a.momentohq.com".to_string(),
        )
        .expect("should parse endpoint");
        assert_eq!("cell-us-east-1-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint(
            "https://api.cache.cell-4-us-west-2-1.prod.a.momentohq.com".to_string(),
        )
        .expect("should parse endpoint");
        assert_eq!("cell-4-us-west-2-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint(
            "https://api.cache.cell-1-ap-southeast-2-1.prod.a.momentohq.com".to_string(),
        )
        .expect("should parse endpoint");
        assert_eq!("cell-1-ap-southeast-2-1.prod.a.momentohq.com", endpoint);
    }

    #[test]
    fn determine_endpoint_with_region_only() {
        let endpoint = determine_endpoint("us-east-1".to_string()).expect("should parse endpoint");
        assert_eq!("cell-us-east-1-1.prod.a.momentohq.com", endpoint);

        let endpoint = determine_endpoint("us-west-2".to_string()).expect("should parse endpoint");
        assert_eq!("cell-4-us-west-2-1.prod.a.momentohq.com", endpoint);

        let endpoint =
            determine_endpoint("ap-southeast-2".to_string()).expect("should parse endpoint");
        assert_eq!("cell-1-ap-southeast-2-1.prod.a.momentohq.com", endpoint);
    }

    #[test]
    fn determine_endpoint_with_invalid_structure() {
        let docs = "https://docs.momentohq.com/platform/regions";

        let error =
            determine_endpoint("N. Virginia".to_string()).expect_err("should reject endpoint");
        assert!(error.contains(docs));

        let error =
            determine_endpoint("cell-1-ap-southeast-1-1.foobar.a.momentohq.com".to_string())
                .expect_err("should reject endpoint");
        assert!(error.contains(docs));
    }
}

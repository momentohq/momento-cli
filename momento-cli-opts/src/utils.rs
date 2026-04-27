use clap::builder::TypedValueParser;
use clap::error::{ContextKind, ContextValue, ErrorKind};

use chrono::NaiveDate;

#[derive(Clone)]
pub struct DateValueParser;

impl TypedValueParser for DateValueParser {
    type Value = NaiveDate;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value.to_str().unwrap_or_default();
        NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
            let mut err = clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
            err.insert(
                ContextKind::InvalidArg,
                ContextValue::String(arg.map(ToString::to_string).unwrap_or_default()),
            );
            err.insert(
                ContextKind::InvalidValue,
                ContextValue::String(value.to_string()),
            );
            err
        })
    }
}

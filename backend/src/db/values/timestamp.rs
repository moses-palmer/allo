use std::error;
use std::fmt;
use std::str;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

/// A wrapper for timestamps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timestamp(pub DateTime<FixedOffset>);

impl Timestamp {
    /// Creates a new timestamp for the current time.
    pub fn now() -> Self {
        Self(Utc::now().into())
    }
}

impl From<DateTime<FixedOffset>> for Timestamp {
    fn from(source: DateTime<FixedOffset>) -> Self {
        Self(source)
    }
}

impl str::FromStr for Timestamp {
    type Err = TimestampParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Ok(Self(DateTime::parse_from_rfc3339(source)?))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.to_rfc3339(),)
    }
}

impl<'a> Deserialize<'a> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug)]
pub struct TimestampParseError(chrono::format::ParseError);

impl fmt::Display for TimestampParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl error::Error for TimestampParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<chrono::format::ParseError> for TimestampParseError {
    fn from(source: chrono::format::ParseError) -> Self {
        Self(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!(
            "1970-01-01T00:00:00+00:00".parse::<Timestamp>().unwrap(),
            DateTime::<FixedOffset>::from(
                DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap()
            )
            .into(),
        );
    }

    #[test]
    fn to_str() {
        let source = "1970-01-01T00:00:00+00:00";

        let a = source.parse::<Timestamp>().unwrap();
        assert_eq!(a.to_string(), source);
    }
}

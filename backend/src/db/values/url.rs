use std::error;
use std::fmt;
use std::str;

use serde::{Deserialize, Serialize};
use url::Url;

/// A URL.
#[derive(Clone, Debug, PartialEq)]
pub struct URL(Url);

impl From<Url> for URL {
    fn from(source: Url) -> Self {
        Self(source)
    }
}

impl str::FromStr for URL {
    type Err = URLParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Ok(Url::parse(source)?.into())
    }
}

impl fmt::Display for URL {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> Deserialize<'a> for URL {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for URL {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug)]
pub struct URLParseError(url::ParseError);

impl fmt::Display for URLParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl error::Error for URLParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<url::ParseError> for URLParseError {
    fn from(source: url::ParseError) -> Self {
        Self(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!(
            "https://example.com/some/path".parse::<URL>().unwrap(),
            Url::parse("https://example.com/some/path").unwrap().into(),
        );
    }

    #[test]
    fn to_str() {
        let source = "https://example.com/some/path";

        let a = source.parse::<URL>().unwrap();
        assert_eq!(a.to_string(), source);
    }
}

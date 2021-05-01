use std::error;
use std::fmt;
use std::str;

use serde::{Deserialize, Serialize};

/// A validated email address.
#[derive(Clone, Debug, PartialEq)]
pub struct EmailAddress(email_address::EmailAddress);

impl From<email_address::EmailAddress> for EmailAddress {
    fn from(source: email_address::EmailAddress) -> Self {
        Self(source)
    }
}

impl str::FromStr for EmailAddress {
    type Err = EmailAddressParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Ok(Self(source.parse()?))
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> Deserialize<'a> for EmailAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for EmailAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug)]
pub struct EmailAddressParseError(email_address::Error);

impl fmt::Display for EmailAddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl error::Error for EmailAddressParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<email_address::Error> for EmailAddressParseError {
    fn from(source: email_address::Error) -> Self {
        Self(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!(
            "test@example.com".parse::<EmailAddress>().unwrap(),
            "test@example.com"
                .parse::<email_address::EmailAddress>()
                .unwrap()
                .into(),
        );
    }

    #[test]
    fn to_str() {
        let source = "test@example.com";

        let a = source.parse::<EmailAddress>().unwrap();
        assert_eq!(a.to_string(), source);
    }
}

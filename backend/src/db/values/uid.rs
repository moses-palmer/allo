use std::error;
use std::fmt;
use std::str;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A wrapper for unique ID's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UID(Uuid);

impl UID {
    /// Creates a new random UID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl str::FromStr for UID {
    type Err = UIDParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(source)?))
    }
}

impl fmt::Display for UID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0.to_simple().encode_lower(&mut Uuid::encode_buffer())
        )
    }
}

impl<'a> Deserialize<'a> for UID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for UID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug)]
pub struct UIDParseError(uuid::Error);

impl fmt::Display for UIDParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl error::Error for UIDParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<uuid::Error> for UIDParseError {
    fn from(source: uuid::Error) -> Self {
        Self(source)
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn from_str() {
        let source = "0123456789abcdef0123456789abcdef";

        assert_eq!(
            source.parse::<UID>().unwrap(),
            UID(Uuid::parse_str(source).unwrap()),
        );
    }

    #[test]
    fn to_str() {
        let source = "0123456789abcdef0123456789abcdef";

        let a = source.parse::<UID>().unwrap();
        assert_eq!(a.to_string(), source);
    }
}

use std::error;
use std::fmt;
use std::str;

use serde::{Deserialize, Serialize};

/// The role of a user in a family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    /// A child in a family.
    Child,

    /// A parent in a family.
    Parent,
}

impl str::FromStr for Role {
    type Err = RoleParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        use Role::*;
        match source {
            "child" => Ok(Child),
            "parent" => Ok(Parent),
            s => Err(RoleParseError(s.into())),
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Role::*;
        match self {
            Child => write!(f, "child"),
            Parent => write!(f, "parent"),
        }
    }
}

impl<'a> Deserialize<'a> for Role {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, PartialEq)]
pub struct RoleParseError(String);

impl fmt::Display for RoleParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid role: {}", self.0)
    }
}

impl error::Error for RoleParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        use Role::*;
        assert_eq!("child".parse::<Role>().unwrap(), Child);
        assert_eq!("parent".parse::<Role>().unwrap(), Parent);
        assert_eq!(
            "unknown".parse::<Role>(),
            Err(RoleParseError("unknown".into())),
        );
    }

    #[test]
    fn to_str() {
        for source in ["child", "parent"].iter() {
            let a = source.parse::<Role>().unwrap();
            assert_eq!(&a.to_string(), source);
        }
    }
}

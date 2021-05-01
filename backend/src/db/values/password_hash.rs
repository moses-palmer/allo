use std::error;
use std::fmt;
use std::str;

use bcrypt::HashParts;
use serde::{Deserialize, Serialize};

/// A wrapper for password hashes.
#[derive(Debug, PartialEq)]
pub struct PasswordHash(HashParts);

impl Clone for PasswordHash {
    fn clone(&self) -> Self {
        Self(self.0.to_string().parse().unwrap())
    }
}

impl PasswordHash {
    /// The cost to use when hashing passwords.
    pub const COST: u32 = bcrypt::DEFAULT_COST;

    /// Generates a password hash from a password.
    ///
    /// This constructor returns nothing iff the password contains a *NUL* byte.
    ///
    /// # Arguments
    /// *  `password` - The password to hash.
    pub fn from_password(password: &str) -> Option<Self> {
        // hash_with_result fails iff the cost is invalid, or
        Some(Self(bcrypt::hash_with_result(password, Self::COST).ok()?))
    }

    /// Attempts to verify a password.
    ///
    /// # Arguments
    /// *  `password` - The password to verify.
    pub fn verify(&self, password: &str) -> Option<bool> {
        bcrypt::verify(&password, &self.0.to_string()).ok()
    }
}

impl From<HashParts> for PasswordHash {
    fn from(source: HashParts) -> Self {
        Self(source)
    }
}

impl str::FromStr for PasswordHash {
    type Err = PasswordHashParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Ok(Self(source.parse()?))
    }
}

impl fmt::Display for PasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl<'a> Deserialize<'a> for PasswordHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for PasswordHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug)]
pub struct PasswordHashParseError(bcrypt::BcryptError);

impl fmt::Display for PasswordHashParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl error::Error for PasswordHashParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<bcrypt::BcryptError> for PasswordHashParseError {
    fn from(source: bcrypt::BcryptError) -> Self {
        Self(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert!(
            "$2y$12$Hj.k7dVs73EiptZR2x6IEuwyNHy4bS/IqowvGpBoqYkJvKrhLKhvy"
                .parse::<PasswordHash>()
                .is_ok()
        );
        assert!("invalid".parse::<PasswordHash>().is_err());
    }

    #[test]
    fn roundtrip() {
        assert_eq!(
            PasswordHash::from_password("password")
                .unwrap()
                .to_string()
                .parse::<PasswordHash>()
                .unwrap()
                .verify("password"),
            Some(true),
        );
    }

    #[test]
    fn verify() {
        assert_eq!(
            "$2y$12$Hj.k7dVs73EiptZR2x6IEuwyNHy4bS/IqowvGpBoqYkJvKrhLKhvy"
                .parse::<PasswordHash>()
                .unwrap()
                .verify("password"),
            Some(true),
        );
        assert_eq!(
            PasswordHash::from_password("password")
                .unwrap()
                .verify("password"),
            Some(true),
        );
        assert_eq!(
            PasswordHash::from_password("password")
                .unwrap()
                .verify("invalid"),
            Some(false),
        );
    }
}

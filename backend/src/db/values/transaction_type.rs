use std::error;
use std::fmt;
use std::str;

use serde::{Deserialize, Serialize};

/// The type of a transaction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransactionType {
    /// The transaction is an allowance payment.
    Allowance,

    /// The transaction is a gift from a parent.
    Gift,

    /// The transaction is a request that has been granted.
    Request,
}

impl str::FromStr for TransactionType {
    type Err = TransactionTypeParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        use TransactionType::*;
        match source {
            "allowance" => Ok(Allowance),
            "gift" => Ok(Gift),
            "request" => Ok(Request),
            s => Err(TransactionTypeParseError(s.into())),
        }
    }
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use TransactionType::*;
        match self {
            Allowance => write!(f, "allowance"),
            Gift => write!(f, "gift"),
            Request => write!(f, "request"),
        }
    }
}

impl<'a> Deserialize<'a> for TransactionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for TransactionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, PartialEq)]
pub struct TransactionTypeParseError(String);

impl fmt::Display for TransactionTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid transaction type: {}", self.0)
    }
}

impl error::Error for TransactionTypeParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        use TransactionType::*;
        assert_eq!("allowance".parse::<TransactionType>().unwrap(), Allowance);
        assert_eq!("gift".parse::<TransactionType>().unwrap(), Gift);
        assert_eq!("request".parse::<TransactionType>().unwrap(), Request);
        assert_eq!(
            "unknown".parse::<TransactionType>(),
            Err(TransactionTypeParseError("unknown".into())),
        );
    }

    #[test]
    fn to_str() {
        for source in ["allowance", "gift", "request"].iter() {
            let a = source.parse::<TransactionType>().unwrap();
            assert_eq!(&a.to_string(), source);
        }
    }
}

use std::error;
use std::fmt;
use std::str;

use serde::{Deserialize, Serialize};

/// The format to use to print a value in a currency.
///
/// It consist of a prefix and a suffix.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CurrencyFormat(String, String);

impl CurrencyFormat {
    /// The separator to use between the prefix and suffix.
    pub const SEPARATOR: &'static str = "{}";

    /// Creates a new currency format.
    ///
    /// If the format string does not contain [`SEPARATOR`](Self::SEPARATOR),
    /// the entire format string is treated as prefix.
    ///
    /// # Argument
    /// *  `s` - The format string.
    pub fn new(s: &str) -> Self {
        if let Some(pos) = s.find(Self::SEPARATOR) {
            Self(s[0..pos].into(), s[pos + Self::SEPARATOR.len()..].into())
        } else {
            Self(s.into(), "".into())
        }
    }
}

impl str::FromStr for CurrencyFormat {
    type Err = CurrencyFormatParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(source))
    }
}

impl fmt::Display for CurrencyFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{{}}{}", self.0, self.1)
    }
}

#[derive(Debug)]
pub struct CurrencyFormatParseError;

impl fmt::Display for CurrencyFormatParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid format")
    }
}

impl error::Error for CurrencyFormatParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!(
            "#{}".parse::<CurrencyFormat>().unwrap(),
            CurrencyFormat::new("#{}"),
        );
    }

    #[test]
    fn to_str() {
        let source = "#{}";

        let a = source.parse::<CurrencyFormat>().unwrap();
        assert_eq!(a.to_string(), source);
    }
}

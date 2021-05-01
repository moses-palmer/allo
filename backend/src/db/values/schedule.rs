use std::error;
use std::fmt;
use std::str;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

/// A Schedule.
#[derive(Clone, Debug, PartialEq)]
pub struct Schedule(pub Weekday);

impl From<Weekday> for Schedule {
    fn from(source: Weekday) -> Self {
        Self(source)
    }
}

impl str::FromStr for Schedule {
    type Err = ScheduleParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            source
                .parse()
                .map_err(|_| ScheduleParseError(source.into()))?,
        ))
    }
}

impl fmt::Display for Schedule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<'a> Deserialize<'a> for Schedule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for Schedule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, PartialEq)]
pub struct ScheduleParseError(String);

impl fmt::Display for ScheduleParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl error::Error for ScheduleParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!("Mon".parse::<Schedule>().unwrap(), Schedule(Weekday::Mon));
        assert_eq!(
            "Monday".parse::<Schedule>().unwrap(),
            Schedule(Weekday::Mon)
        );
        assert_eq!("Tue".parse::<Schedule>().unwrap(), Schedule(Weekday::Tue));
        assert_eq!(
            "Tuesday".parse::<Schedule>().unwrap(),
            Schedule(Weekday::Tue)
        );
        assert_eq!("Wed".parse::<Schedule>().unwrap(), Schedule(Weekday::Wed));
        assert_eq!(
            "Wednesday".parse::<Schedule>().unwrap(),
            Schedule(Weekday::Wed)
        );
        assert_eq!("Thu".parse::<Schedule>().unwrap(), Schedule(Weekday::Thu));
        assert_eq!(
            "Thursday".parse::<Schedule>().unwrap(),
            Schedule(Weekday::Thu)
        );
        assert_eq!("Fri".parse::<Schedule>().unwrap(), Schedule(Weekday::Fri));
        assert_eq!(
            "Friday".parse::<Schedule>().unwrap(),
            Schedule(Weekday::Fri)
        );
        assert_eq!("Sat".parse::<Schedule>().unwrap(), Schedule(Weekday::Sat));
        assert_eq!(
            "Saturday".parse::<Schedule>().unwrap(),
            Schedule(Weekday::Sat)
        );
        assert_eq!("Sun".parse::<Schedule>().unwrap(), Schedule(Weekday::Sun));
        assert_eq!(
            "Sunday".parse::<Schedule>().unwrap(),
            Schedule(Weekday::Sun)
        );
        assert_eq!(
            "unknown".parse::<Schedule>(),
            Err(ScheduleParseError("unknown".into())),
        );
    }

    #[test]
    fn to_str() {
        for source in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter() {
            let a = source.parse::<Schedule>().unwrap();
            assert_eq!(&a.to_string(), source);
        }
    }
}

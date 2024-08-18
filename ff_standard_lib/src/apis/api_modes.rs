use std::str::FromStr;
use serde::{Serialize, Serializer};
use serde_derive::Deserialize;

///The mode sets the base api endpoint
///
/// # Use Case
/// Oanda has a separate endpoint for practice and Live accounts
#[derive(Clone, PartialEq, Deserialize, Eq, Hash)]
pub enum Mode {
    Live,
    Practice,
}

impl Serialize for Mode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        match self {
            Mode::Live => serializer.serialize_str("live"),
            Mode::Practice => serializer.serialize_str("practice"),
        }
    }
}

impl FromStr for Mode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "live" => Ok(Mode::Live),
            "practice" => Ok(Mode::Practice),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Mode::Live => write!(f, "live"),
            Mode::Practice => write!(f, "practice"),
        }
    }
}

impl std::fmt::Debug for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Mode::Live => write!(f, "Live"),
            Mode::Practice => write!(f, "Practice"),
        }
    }
}
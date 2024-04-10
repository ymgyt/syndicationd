use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// `Requirement` expresses how important the feed is
/// using an analogy to [RFC2119](https://datatracker.ietf.org/doc/html/rfc2119)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
pub enum Requirement {
    /// `Must` indicates it must be read
    Must,
    /// `Should` suggests it should be read unless there is a special reason not to
    Should,
    /// `May` implies it is probably worth reading
    May,
}

impl FromStr for Requirement {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s.eq_ignore_ascii_case("MUST") => Ok(Requirement::Must),
            _ if s.eq_ignore_ascii_case("SHOULD") => Ok(Requirement::Should),
            _ if s.eq_ignore_ascii_case("MAY") => Ok(Requirement::May),
            _ => Err("invalid requirement, should be one of ['must', 'should', 'may']"),
        }
    }
}

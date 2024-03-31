use nom::Finish as _;

use crate::application::parse_requirement;
use std::str::FromStr;

/// `Requirement` expresses how important the feed is
/// using an analogy to [RFC2119](https://datatracker.ietf.org/doc/html/rfc2119)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        parse_requirement(s)
            .finish()
            .map(|(_, r)| r)
            .map_err(|_| "invalid requirement, should be one of ['must', 'should', 'may']")
    }
}

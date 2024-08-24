use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

/// `Requirement` expresses how important the feed is
/// using an analogy to [RFC2119](https://datatracker.ietf.org/doc/html/rfc2119)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[cfg_attr(feature = "fake", derive(fake::Dummy))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub enum Requirement {
    /// `Must` indicates it must be read
    Must = 2,
    /// `Should` suggests it should be read unless there is a special reason not to
    Should = 1,
    /// `May` implies it is probably worth reading
    May = 0,
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

impl Display for Requirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Requirement::Must => f.pad("MUST"),
            Requirement::Should => f.pad("SHOULD"),
            Requirement::May => f.pad("MAY"),
        }
    }
}

impl Requirement {
    #[must_use]
    pub fn up(self) -> Self {
        Requirement::from_num((self as isize).saturating_add(1))
    }

    #[must_use]
    pub fn down(self) -> Self {
        #[allow(clippy::match_same_arms)]
        let n = self as isize;
        Requirement::from_num(n.saturating_sub(1))
    }

    pub fn is_satisfied(self, condition: Requirement) -> bool {
        (self as isize) >= (condition as isize)
    }

    fn from_num(n: isize) -> Self {
        match n {
            2.. => Requirement::Must,
            1 => Requirement::Should,
            _ => Requirement::May,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!("must".parse(), Ok(Requirement::Must));
        assert_eq!("should".parse(), Ok(Requirement::Should));
        assert_eq!("may".parse(), Ok(Requirement::May));
        assert!("unexpected".parse::<Requirement>().is_err());
    }
}

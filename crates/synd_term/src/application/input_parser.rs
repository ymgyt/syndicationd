use nom_language::error::{VerboseError, VerboseErrorKind};
use synd_client::payload::SubscribeFeedInput;
use thiserror::Error;

use crate::{
    config::Categories,
    types::{self, RefreshPolicyExt},
};

type NomError<'s> = VerboseError<&'s str>;

const CTX_REQUIREMENT: &str = "requirement";
const CTX_CATEGORY: &str = "category";
const CTX_CATEGORY_POST: &str = "category_post";
const CTX_URL: &str = "url";
const CTX_REFRESH_POLICY: &str = "refresh_policy";

#[derive(Error, Debug, PartialEq, Eq)]
pub(super) enum ParseFeedError {
    #[error("parse feed error: {0}")]
    Parse(String),
}

pub(super) struct InputParser<'a> {
    input: &'a str,
}

impl<'a> InputParser<'a> {
    pub(super) const SUSBSCRIBE_FEED_PROMPT: &'static str =
        "# Please enter the requirement, category, and URL for subscription in the following format
#
# <requirement> <category> <url> [manual|interval:<duration>]
#
#   * The requirement must be one of 
#     * \"MUST\" 
#     * \"SHOULD\" 
#     * \"MAY\"
#   * For the category, please choose one category of the feed(for example, \"rust\")
#   * Refresh policy is optional. Use \"manual\" or \"interval:2h\".
#
# with '#' will be ignored, and an empty URL aborts the subscription.
#
# Example:
# MUST rust https://this-week-in-rust.org/atom.xml interval:2h
";

    pub(super) fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub(super) fn parse_feed_subscription(
        &self,
        categories: &Categories,
    ) -> Result<SubscribeFeedInput, ParseFeedError> {
        feed::parse(self.input)
            .map(|mut input| {
                if let Some(category) = input.category {
                    input.category = Some(categories.normalize(category));
                }
                input
            })
            .map_err(|mut verbose_err: NomError| {
                let msg = match verbose_err.errors.pop() {
                    Some((input, VerboseErrorKind::Context(CTX_REQUIREMENT))) => {
                        format!(
                            "Invalid requirement: must be one of 'MUST' 'SHOULD' 'MAY'. {input}"
                        )
                    }
                    Some((input, VerboseErrorKind::Context(CTX_CATEGORY_POST))) => {
                        format!("Invalid category: {input}")
                    }
                    Some((input, VerboseErrorKind::Context(CTX_URL))) => {
                        format!("Invalid url: {input}")
                    }
                    Some((input, VerboseErrorKind::Context(CTX_REFRESH_POLICY))) => {
                        format!(
                            "Invalid refresh policy: use 'manual' or 'interval:<duration>'. {input}"
                        )
                    }
                    Some((input, _)) => format!("Failed to parse input: {input}"),
                    None => "Failed to parse input".to_owned(),
                };
                ParseFeedError::Parse(msg)
            })
    }

    pub(super) fn edit_feed_prompt(feed: &types::Feed) -> String {
        let refresh_policy = feed
            .refresh_policy
            .prompt_value()
            .map(|policy| format!(" {policy}"))
            .unwrap_or_default();

        format!(
            "{}\n{requirement} {category} {feed_url}{refresh_policy}",
            Self::SUSBSCRIBE_FEED_PROMPT,
            requirement = feed.requirement(),
            category = feed.category(),
            feed_url = feed.url,
        )
    }
}

mod feed {
    use nom::{
        AsChar, Finish, IResult, Parser,
        branch::alt,
        bytes::complete::{tag_no_case, take_while, take_while_m_n},
        character::complete::{multispace0, multispace1},
        combinator::{all_consuming, map, opt, value},
        error::context,
        sequence::delimited,
    };
    use nom_language::error::{VerboseError, VerboseErrorKind};
    use synd_client::payload::{RefreshPolicyInput, RefreshPolicyInputKind, SubscribeFeedInput};
    use synd_feed::types::{Category, FeedUrl, Requirement};
    use url::Url;

    use super::NomError;
    use crate::application::input_parser::{
        CTX_CATEGORY, CTX_CATEGORY_POST, CTX_REFRESH_POLICY, CTX_REQUIREMENT, CTX_URL, comment,
    };

    pub(super) fn parse(s: &'_ str) -> Result<SubscribeFeedInput, NomError<'_>> {
        all_consuming(delimited(comment::comments, feed_input, comment::comments))
            .parse(s)
            .finish()
            .map(|(_, input)| input)
    }

    fn feed_input(s: &'_ str) -> IResult<&'_ str, SubscribeFeedInput, NomError<'_>> {
        let (remain, (_, requirement, _, category, _, feed_url, refresh_policy, _)) = (
            multispace0,
            requirement,
            multispace1,
            category,
            context(CTX_CATEGORY_POST, multispace1),
            url,
            opt((multispace1, refresh_policy).map(|(_, policy)| policy)),
            multispace0,
        )
            .parse(s)?;
        Ok((
            remain,
            SubscribeFeedInput {
                url: feed_url,
                requirement: Some(requirement),
                category: Some(category),
                refresh_policy,
            },
        ))
    }

    pub fn requirement(s: &'_ str) -> IResult<&'_ str, Requirement, NomError<'_>> {
        context(
            CTX_REQUIREMENT,
            alt((
                value(Requirement::Must, tag_no_case("MUST")),
                value(Requirement::Should, tag_no_case("SHOULD")),
                value(Requirement::May, tag_no_case("MAY")),
            )),
        )
        .parse(s)
    }

    fn category(s: &'_ str) -> IResult<&'_ str, Category<'static>, NomError<'_>> {
        let (remain, category) = context(
            CTX_CATEGORY,
            take_while_m_n(1, 20, |c: char| c.is_alphanum()),
        )
        .parse(s)?;

        Ok((
            remain,
            Category::new(category.to_owned()).expect("this is a bug"),
        ))
    }

    fn url(s: &'_ str) -> IResult<&'_ str, FeedUrl, NomError<'_>> {
        let (remain, url) = context(
            CTX_URL,
            map(take_while(|c: char| !c.is_whitespace()), |s: &str| {
                s.to_owned()
            }),
        )
        .parse(s)?;
        match Url::parse(&url) {
            Ok(url) => Ok((remain, FeedUrl::from(url))),
            Err(err) => {
                tracing::warn!("Invalid url: {err}");
                let nom_err = VerboseError {
                    errors: vec![(s, VerboseErrorKind::Context("url"))],
                };
                Err(nom::Err::Failure(nom_err))
            }
        }
    }

    fn refresh_policy(s: &'_ str) -> IResult<&'_ str, RefreshPolicyInput, NomError<'_>> {
        let (remain, token) = context(
            CTX_REFRESH_POLICY,
            take_while_m_n(1, 64, |c: char| !c.is_whitespace()),
        )
        .parse(s)?;

        if token.eq_ignore_ascii_case("manual") {
            return Ok((
                remain,
                RefreshPolicyInput {
                    kind: RefreshPolicyInputKind::Manual,
                    interval_seconds: None,
                },
            ));
        }

        let Some((kind, duration)) = token.split_once(':') else {
            return Err(invalid_refresh_policy(s));
        };
        if !kind.eq_ignore_ascii_case("interval") {
            return Err(invalid_refresh_policy(s));
        }

        let duration = synd_support::time::humantime::parse_duration(duration)
            .map_err(|_| invalid_refresh_policy(s))?;
        if duration.subsec_nanos() != 0 {
            return Err(invalid_refresh_policy(s));
        }
        let seconds = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
        if seconds == 0 {
            return Err(invalid_refresh_policy(s));
        }

        Ok((
            remain,
            RefreshPolicyInput {
                kind: RefreshPolicyInputKind::Interval,
                interval_seconds: Some(seconds),
            },
        ))
    }

    fn invalid_refresh_policy(input: &'_ str) -> nom::Err<NomError<'_>> {
        nom::Err::Failure(VerboseError {
            errors: vec![(input, VerboseErrorKind::Context(CTX_REFRESH_POLICY))],
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_requirement() {
            assert_eq!(requirement("must"), Ok(("", Requirement::Must)));
            assert_eq!(requirement("Must"), Ok(("", Requirement::Must)));
            assert_eq!(requirement("MUST"), Ok(("", Requirement::Must)));
            assert_eq!(requirement("should"), Ok(("", Requirement::Should)));
            assert_eq!(requirement("Should"), Ok(("", Requirement::Should)));
            assert_eq!(requirement("SHOULD"), Ok(("", Requirement::Should)));
            assert_eq!(requirement("may"), Ok(("", Requirement::May)));
            assert_eq!(requirement("May"), Ok(("", Requirement::May)));
            assert_eq!(requirement("MAY"), Ok(("", Requirement::May)));
        }

        #[test]
        fn parse_category() {
            assert_eq!(category("rust"), Ok(("", Category::new("rust").unwrap())));
            assert_eq!(category("Rust"), Ok(("", Category::new("rust").unwrap())));
        }

        #[test]
        fn parse_feed_input() {
            assert_eq!(
                feed_input("MUST rust https://example.ymgyt.io/atom.xml"),
                Ok((
                    "",
                    SubscribeFeedInput {
                        url: "https://example.ymgyt.io/atom.xml".try_into().unwrap(),
                        requirement: Some(Requirement::Must),
                        category: Some(Category::new("rust").unwrap()),
                        refresh_policy: None,
                    }
                ))
            );
        }

        #[test]
        fn parse_feed_input_with_manual_refresh_policy() {
            assert_eq!(
                feed_input("MUST rust https://example.ymgyt.io/atom.xml manual"),
                Ok((
                    "",
                    SubscribeFeedInput {
                        url: "https://example.ymgyt.io/atom.xml".try_into().unwrap(),
                        requirement: Some(Requirement::Must),
                        category: Some(Category::new("rust").unwrap()),
                        refresh_policy: Some(RefreshPolicyInput {
                            kind: RefreshPolicyInputKind::Manual,
                            interval_seconds: None,
                        }),
                    }
                ))
            );
        }

        #[test]
        fn parse_feed_input_with_interval_refresh_policy() {
            assert_eq!(
                feed_input("MUST rust https://example.ymgyt.io/atom.xml interval:30m"),
                Ok((
                    "",
                    SubscribeFeedInput {
                        url: "https://example.ymgyt.io/atom.xml".try_into().unwrap(),
                        requirement: Some(Requirement::Must),
                        category: Some(Category::new("rust").unwrap()),
                        refresh_policy: Some(RefreshPolicyInput {
                            kind: RefreshPolicyInputKind::Interval,
                            interval_seconds: Some(1800),
                        }),
                    }
                ))
            );
        }

        #[test]
        fn parse_feed_input_error() {
            let tests = vec![
                (
                    "foo rust https://example.ymgyt.io/atom.xml",
                    CTX_REQUIREMENT,
                ),
                (
                    "should https://example.ymgyt.io/atom.xml",
                    CTX_CATEGORY_POST,
                ),
                (
                    "should rust https://example.ymgyt.io/atom.xml interval:0s",
                    CTX_REFRESH_POLICY,
                ),
                (
                    "should rust https://example.ymgyt.io/atom.xml interval:1500ms",
                    CTX_REFRESH_POLICY,
                ),
            ];

            for test in tests {
                let (_, kind) = feed_input(test.0)
                    .finish()
                    .unwrap_err()
                    .errors
                    .pop()
                    .unwrap();
                assert_eq!(kind, VerboseErrorKind::Context(test.1));
            }

            let err = feed_input("should https://example.ymgyt.io/atom.xml")
                .finish()
                .unwrap_err()
                .errors;
            println!("{err:?}");
        }

        #[test]
        fn parse_rejects_trailing_garbage() {
            let err = parse("MUST rust https://example.ymgyt.io/atom.xml manual extra")
                .unwrap_err()
                .errors;
            assert!(!err.is_empty());
        }
    }
}

mod comment {
    use nom::{
        IResult, Parser,
        bytes::complete::{tag, take_until},
        character::complete::line_ending,
        combinator::value,
        multi::fold_many0,
        sequence::delimited,
    };

    use crate::application::input_parser::NomError;

    pub(super) fn comments(s: &'_ str) -> IResult<&'_ str, (), NomError<'_>> {
        fold_many0(comment, || (), |acc, ()| acc).parse(s)
    }

    pub(super) fn comment(s: &'_ str) -> IResult<&'_ str, (), NomError<'_>> {
        value((), delimited(tag("#"), take_until("\n"), line_ending)).parse(s)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_comment() {
            assert_eq!(comment("# foo\n"), Ok(("", ())));
            assert_eq!(comment("# foo\r\n"), Ok(("", ())));
        }

        #[test]
        fn parse_comments() {
            let s = "# comment1\n# comment2\n";
            assert_eq!(comments(s), Ok(("", ())));
        }
    }
}

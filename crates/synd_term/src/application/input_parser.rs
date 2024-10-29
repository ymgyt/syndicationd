use nom::error::VerboseErrorKind;
use thiserror::Error;

use crate::{
    client::synd_api::mutation::subscribe_feed::SubscribeFeedInput,
    config::Categories,
    types::{self},
};

type NomError<'s> = nom::error::VerboseError<&'s str>;

const CTX_REQUIREMENT: &str = "requirement";
const CTX_CATEGORY: &str = "category";
const CTX_CATEGORY_POST: &str = "category_post";
const CTX_URL: &str = "url";

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
# <requirement> <category> <url>
#
#   * The requirement must be one of 
#     * \"MUST\" 
#     * \"SHOULD\" 
#     * \"MAY\"
#   * For the category, please choose one category of the feed(for example, \"rust\")
#
# with '#' will be ignored, and an empty URL aborts the subscription.
#
# Example:
# MUST rust https://this-week-in-rust.org/atom.xml
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
                        format!("Invalid category: {input}",)
                    }
                    Some((input, VerboseErrorKind::Context(CTX_URL))) => {
                        format!("Invalid url: {input}")
                    }
                    Some((input, _)) => format!("Failed to parse input: {input}"),
                    None => "Failed to parse input".to_owned(),
                };
                ParseFeedError::Parse(msg)
            })
    }

    pub(super) fn edit_feed_prompt(feed: &types::Feed) -> String {
        format!(
            "{}\n{requirement} {category} {feed_url}",
            Self::SUSBSCRIBE_FEED_PROMPT,
            requirement = feed.requirement(),
            category = feed.category(),
            feed_url = feed.url,
        )
    }
}

mod feed {
    use nom::{
        branch::alt,
        bytes::complete::{tag_no_case, take_while, take_while_m_n},
        character::complete::{multispace0, multispace1},
        combinator::{map, value},
        error::context,
        sequence::delimited,
        AsChar, Finish, IResult, Parser,
    };
    use synd_feed::types::{Category, FeedUrl};
    use url::Url;

    use super::NomError;
    use crate::{
        application::input_parser::{
            comment, CTX_CATEGORY, CTX_CATEGORY_POST, CTX_REQUIREMENT, CTX_URL,
        },
        client::synd_api::mutation::subscribe_feed::{Requirement, SubscribeFeedInput},
    };

    pub(super) fn parse(s: &str) -> Result<SubscribeFeedInput, NomError> {
        delimited(comment::comments, feed_input, comment::comments)
            .parse(s)
            .finish()
            .map(|(_, input)| input)
    }

    fn feed_input(s: &str) -> IResult<&str, SubscribeFeedInput, NomError> {
        let (remain, (_, requirement, _, category, _, feed_url, _)) = (
            multispace0,
            requirement,
            multispace1,
            category,
            context(CTX_CATEGORY_POST, multispace1),
            url,
            multispace0,
        )
            .parse(s)?;
        Ok((
            remain,
            SubscribeFeedInput {
                url: feed_url,
                requirement: Some(requirement),
                category: Some(category),
            },
        ))
    }

    pub fn requirement(s: &str) -> IResult<&str, Requirement, NomError> {
        context(
            CTX_REQUIREMENT,
            alt((
                value(Requirement::MUST, tag_no_case("MUST")),
                value(Requirement::SHOULD, tag_no_case("SHOULD")),
                value(Requirement::MAY, tag_no_case("MAY")),
            )),
        )
        .parse(s)
    }

    fn category(s: &str) -> IResult<&str, Category<'static>, NomError> {
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

    fn url(s: &str) -> IResult<&str, FeedUrl, NomError> {
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
                let nom_err = nom::error::VerboseError {
                    errors: vec![(s, nom::error::VerboseErrorKind::Context("url"))],
                };
                Err(nom::Err::Failure(nom_err))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use nom::error::VerboseErrorKind;

        use super::*;

        #[test]
        fn parse_requirement() {
            assert_eq!(requirement("must"), Ok(("", Requirement::MUST)));
            assert_eq!(requirement("Must"), Ok(("", Requirement::MUST)));
            assert_eq!(requirement("MUST"), Ok(("", Requirement::MUST)));
            assert_eq!(requirement("should"), Ok(("", Requirement::SHOULD)));
            assert_eq!(requirement("Should"), Ok(("", Requirement::SHOULD)));
            assert_eq!(requirement("SHOULD"), Ok(("", Requirement::SHOULD)));
            assert_eq!(requirement("may"), Ok(("", Requirement::MAY)));
            assert_eq!(requirement("May"), Ok(("", Requirement::MAY)));
            assert_eq!(requirement("MAY"), Ok(("", Requirement::MAY)));
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
                        requirement: Some(Requirement::MUST),
                        category: Some(Category::new("rust").unwrap())
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
    }
}

mod comment {
    use nom::{
        bytes::complete::{tag, take_until},
        character::complete::line_ending,
        combinator::value,
        multi::fold_many0,
        sequence::delimited,
        IResult, Parser,
    };

    use crate::application::input_parser::NomError;

    pub(super) fn comments(s: &str) -> IResult<&str, (), NomError> {
        fold_many0(comment, || (), |acc, ()| acc).parse(s)
    }

    pub(super) fn comment(s: &str) -> IResult<&str, (), NomError> {
        value((), delimited(tag("#"), take_until("\n"), line_ending)).parse(s)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_comment() {
            assert_eq!(comment("# foo\n"), Ok(("", ())),);
            assert_eq!(comment("# foo\r\n"), Ok(("", ())),);
        }

        #[test]
        fn parse_comments() {
            let s = "# comment1\n# comment2\n";
            assert_eq!(comments(s), Ok(("", ())));
        }
    }
}

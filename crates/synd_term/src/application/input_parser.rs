use thiserror::Error;

use crate::{
    client::mutation::subscribe_feed::SubscribeFeedInput,
    types::{self},
};

pub use feed::requirement as parse_requirement;

type NomError<'s> = nom::error::Error<&'s str>;

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
#   * The requirement must be one of \"MUST\", \"SHOULD\", \"MAY\"
#   * For the category, please choose one category of the feed(for exampke, \"rust\"
#
# with '#' will be ignored, and an empty URL aborts the subscription.
#
# Example:
# MUST rust https://this-week-in-rust.org/atom.xml
";

    pub(super) fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub(super) fn parse_feed_subscription(&self) -> Result<SubscribeFeedInput, ParseFeedError> {
        feed::parse(self.input).map_err(|e| ParseFeedError::Parse(e.to_string()))
    }

    pub(super) fn edit_feed_prompt(feed: &types::Feed) -> String {
        format!(
            "{}\n{feed_url}",
            Self::SUSBSCRIBE_FEED_PROMPT,
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
        sequence::{delimited, Tuple},
        Finish, IResult, Parser,
    };
    use synd_feed::types::Category;

    use super::NomError;
    use crate::{
        application::input_parser::comment,
        client::mutation::subscribe_feed::{Requirement, SubscribeFeedInput},
    };

    pub(super) fn parse(s: &str) -> Result<SubscribeFeedInput, NomError> {
        delimited(comment::comments, feed_input, comment::comments)
            .parse(s)
            .finish()
            .map(|(_, input)| input)
    }

    fn feed_input(s: &str) -> IResult<&str, SubscribeFeedInput> {
        let (remain, (_, requirement, _, category, _, feed_url, _)) = (
            multispace0,
            requirement,
            multispace1,
            category,
            multispace1,
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

    pub fn requirement(s: &str) -> IResult<&str, Requirement> {
        alt((
            value(Requirement::MUST, tag_no_case("MUST")),
            value(Requirement::SHOULD, tag_no_case("SHOULD")),
            value(Requirement::MAY, tag_no_case("MAY")),
        ))
        .parse(s)
    }

    fn category(s: &str) -> IResult<&str, Category<'static>> {
        let (remain, category) = take_while_m_n(1, 20, |c| c != ' ').parse(s)?;
        Ok((
            remain,
            Category::new(category.to_owned()).expect("this is a bug"),
        ))
    }

    fn url(s: &str) -> IResult<&str, String> {
        map(take_while(|c: char| !c.is_whitespace()), |s: &str| {
            s.to_owned()
        })
        .parse(s)
    }

    #[cfg(test)]
    mod test {
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
                        url: "https://example.ymgyt.io/atom.xml".into(),
                        requirement: Some(Requirement::MUST),
                        category: Some(Category::new("rust").unwrap())
                    }
                ))
            );
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

    pub(super) fn comments(s: &str) -> IResult<&str, ()> {
        fold_many0(comment, || (), |acc, ()| acc).parse(s)
    }

    pub(super) fn comment(s: &str) -> IResult<&str, ()> {
        value((), delimited(tag("#"), take_until("\n"), line_ending)).parse(s)
    }

    #[cfg(test)]
    mod test {
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

#[cfg(test)]
mod test {
    // use super::*;
    // TODO: update test
    /*
    #[test]
    fn parse_feed_url() {
        let prompt = InputParser::SUSBSCRIBE_FEED_PROMPT;
        let cases = vec![
            (
                format!("{prompt}https://blog.ymgyt.io/atom.xml"),
                Ok("https://blog.ymgyt.io/atom.xml"),
            ),
            (
                format!("{prompt}   https://blog.ymgyt.io/atom.xml  "),
                Ok("https://blog.ymgyt.io/atom.xml"),
            ),
            (
                format!("{prompt}\nhttps://blog.ymgyt.io/atom.xml\n"),
                Ok("https://blog.ymgyt.io/atom.xml"),
            ),
        ];

        for case in cases {
            let p = InputParser::new(case.0.as_str());
            assert_eq!(p.parse_feed_subscription(), case.1);
        }
    }
    */
}

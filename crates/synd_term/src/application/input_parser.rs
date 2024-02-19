use thiserror::Error;
use url::Url;

#[derive(Error, Debug, PartialEq, Eq)]
pub(super) enum ParseFeedUrlError {
    #[error("invalid feed url: `{input}`: {err}")]
    InvalidUrl { input: String, err: url::ParseError },
    #[error("no input")]
    NoInput,
}

pub(super) struct InputParser<'a> {
    input: &'a str,
}

impl<'a> InputParser<'a> {
    pub(super) const SUSBSCRIBE_FEED_PROMPT: &'static str =
        "# Please enter the URL of the feed to subscribe.
# with '#' will be ignored, and an empty URL aborts the subscription.
# Example:
# https://this-week-in-rust.org/atom.xml
";
    pub(super) fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub(super) fn parse_feed_url(&self) -> Result<&'a str, ParseFeedUrlError> {
        match self
            .input
            .lines()
            .map(str::trim)
            .filter(|s| !s.starts_with('#') && !s.is_empty())
            .map(|s| (Url::parse(s), s))
            .next()
        {
            Some((Ok(_), input)) => Ok(input),
            Some((Err(parse_err), input)) => Err(ParseFeedUrlError::InvalidUrl {
                input: input.to_owned(),
                err: parse_err,
            }),
            None => Err(ParseFeedUrlError::NoInput),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
            assert_eq!(p.parse_feed_url(), case.1);
        }
    }
}

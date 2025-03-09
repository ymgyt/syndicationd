use crate::{
    Time,
    message::{MessageFrames, MessageType, ParseError, frame::prefix, parse::parse},
};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct Ping {
    pub client_timestamp: Option<Time>,
    pub server_timestamp: Option<Time>,
}

impl Ping {
    pub fn new() -> Self {
        Self {
            client_timestamp: None,
            server_timestamp: None,
        }
    }

    #[must_use]
    pub fn with_client_timestamp(self, time: Time) -> Self {
        Self {
            client_timestamp: Some(time),
            ..self
        }
    }

    #[must_use]
    pub fn with_server_timestamp(self, time: Time) -> Self {
        Self {
            server_timestamp: Some(time),
            ..self
        }
    }
}

impl Default for Ping {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Ping> for MessageFrames {
    fn from(ping: Ping) -> Self {
        let mut frames = MessageFrames::new(MessageType::Ping, 2);

        frames.push_time_or_null(ping.client_timestamp);
        frames.push_time_or_null(ping.server_timestamp);

        frames
    }
}

impl Ping {
    pub(crate) fn parse(input: &[u8]) -> Result<(&[u8], Self), ParseError> {
        let parse_time = |input| -> Result<(&[u8], Option<DateTime<Utc>>), ParseError> {
            let (input, t) = parse::time(input).map_err(|err| ParseError::expect(err, "time"))?;
            let rfc3339 = String::from_utf8(t.to_vec())?;
            let t = DateTime::parse_from_rfc3339(&rfc3339).unwrap();
            Ok((input, Some(t.with_timezone(&Utc))))
        };
        let parse_null = |input| -> Result<(&[u8], Option<DateTime<Utc>>), ParseError> {
            let (input, ()) = parse::null(input).map_err(|err| ParseError::expect(err, "null"))?;
            Ok((input, None))
        };

        let (input, pre) =
            parse::peek_prefix(input).map_err(|err| ParseError::expect(err, "prefix"))?;
        let (input, client) = match pre {
            prefix::TIME => parse_time(input)?,
            prefix::NULL => parse_null(input)?,
            _ => unreachable!(),
        };

        let (input, pre) =
            parse::peek_prefix(input).map_err(|err| ParseError::expect(err, "prefix"))?;
        let (input, server) = match pre {
            prefix::TIME => parse_time(input)?,
            prefix::NULL => parse_null(input)?,
            _ => unreachable!(),
        };
        Ok((
            input,
            Ping {
                client_timestamp: client,
                server_timestamp: server,
            },
        ))
    }
}

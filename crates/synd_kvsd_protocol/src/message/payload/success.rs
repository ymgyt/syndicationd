use crate::{
    message::{frame::prefix, parse::parse, MessageFrames, ParseError},
    Value,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Success {
    value: Option<Value>,
}

impl Success {
    pub fn new() -> Self {
        Self { value: None }
    }

    pub fn with_value(value: Value) -> Self {
        Self { value: Some(value) }
    }
}

impl Default for Success {
    fn default() -> Self {
        Success::new()
    }
}

impl From<Success> for MessageFrames {
    fn from(s: Success) -> Self {
        let mut frames = MessageFrames::new(crate::message::MessageType::Success, 1);

        match s.value {
            Some(value) => frames.push_bytes(value.into_boxed_bytes()),
            None => frames.push_null(),
        }

        frames
    }
}

impl Success {
    pub(crate) fn parse(input: &[u8]) -> Result<(&[u8], Self), ParseError> {
        let (input, pre) =
            parse::peek_prefix(input).map_err(|err| ParseError::expect(err, "prefix"))?;
        let (input, value) = match pre {
            prefix::BYTES => {
                let (input, value) =
                    parse::bytes(input).map_err(|err| ParseError::expect(err, "bytes"))?;
                (input, Some(Value::new_unchecked(value)))
            }
            prefix::NULL => {
                let (input, ()) =
                    parse::null(input).map_err(|err| ParseError::expect(err, "null"))?;
                (input, None)
            }
            _ => unreachable!(),
        };

        let success = Success { value };
        Ok((input, success))
    }
}

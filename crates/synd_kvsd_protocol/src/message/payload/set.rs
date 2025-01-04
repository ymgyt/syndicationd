use crate::{
    message::{parse::parse, MessageFrames, MessageType, ParseError},
    Key, Value,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Set {
    pub(crate) key: Key,
    pub(crate) value: Value,
}

impl Set {
    pub fn new(key: Key, value: Value) -> Self {
        Self { key, value }
    }
}

impl From<Set> for MessageFrames {
    fn from(set: Set) -> Self {
        let mut frames = MessageFrames::new(MessageType::Set, 2);

        frames.push_string(set.key.into_string());
        frames.push_bytes(set.value.into_boxed_bytes());

        frames
    }
}

impl Set {
    pub(crate) fn parse(input: &[u8]) -> Result<(&[u8], Self), ParseError> {
        let (input, key) = parse::string(input).map_err(|err| ParseError::expect(err, "key"))?;
        let key = String::from_utf8(key.to_vec())?;
        let key = Key::new(key).unwrap();
        let (input, value) = parse::bytes(input).map_err(|err| ParseError::expect(err, "value"))?;
        let value = Value::new_unchecked(value);

        Ok((input, Set::new(key, value)))
    }
}

use std::{borrow::Cow, convert::TryFrom, fmt, ops::Deref};

use thiserror::Error;

/// Maximum number of bytes in Key.
/// if it's not in ascii, Len is misleading, so using Bytes explicitly.
pub const MAX_KYE_BYTES: usize = 1024;

/// Maximum number of bytes in Value.
pub const MAX_VALUE_BYTES: usize = 1024 * 1024 * 10;

#[derive(Error, Debug)]
pub enum KeyValueError {
    /// The Key exceeds the maximum number of bytes specified in the protocol.
    #[error("max key bytes exceeded. key: {key} max: {max_bytes}")]
    MaxKeyBytes {
        /// Given key.
        key: String,
        /// Maximum bytes.
        max_bytes: usize,
    },
    /// The value exceeds the maximum number of bytes specified in the protocol.
    #[error("max value bytes exceeded. max: {max_bytes}")]
    MaxValueBytes {
        /// Maximum bytes.
        max_bytes: usize,
    },
}

/// Key represents a string that meets the specifications of the kvsd protocol.
/// other components can handle Key without checking the length.
#[derive(Debug, Clone, PartialEq)]
pub struct Key(String);

impl Deref for Key {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Key {
    type Error = KeyValueError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Key::new(s)
    }
}

impl TryFrom<&str> for Key {
    type Error = KeyValueError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Key::new(s)
    }
}

impl<'a> TryFrom<Cow<'a, str>> for Key {
    type Error = KeyValueError;
    fn try_from(s: Cow<'a, str>) -> Result<Self, Self::Error> {
        Key::new(s)
    }
}

impl Key {
    /// Construct Key from given string.
    pub fn new(s: impl Into<String>) -> Result<Self, KeyValueError> {
        let s = s.into();
        if s.len() > MAX_KYE_BYTES {
            Err(KeyValueError::MaxKeyBytes {
                key: s,
                max_bytes: MAX_KYE_BYTES,
            })
        } else {
            Ok(Self(s))
        }
    }

    /// Convert into String.
    pub fn into_string(self) -> String {
        self.0
    }
}

/// Value represents binary data given by user.
/// It does not have to be Vec<u8> because we do not mutate.
#[derive(Clone, PartialEq)]
pub struct Value(Box<[u8]>);

impl Deref for Value {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Value {
    /// Construct Value.
    /// if given value exceed the maximum bytes, return error.
    pub fn new(v: impl Into<Box<[u8]>>) -> Result<Self, KeyValueError> {
        let v = v.into();
        if v.len() > MAX_VALUE_BYTES {
            Err(KeyValueError::MaxValueBytes {
                max_bytes: MAX_VALUE_BYTES,
            })
        } else {
            Ok(Value(v))
        }
    }

    pub fn new_unchecked(v: impl Into<Box<[u8]>>) -> Self {
        Value(v.into())
    }

    /// Convert into Box<[u8]>
    pub fn into_boxed_bytes(self) -> Box<[u8]> {
        self.0
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl TryFrom<Vec<u8>> for Value {
    type Error = KeyValueError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Value::new(value)
    }
}

impl TryFrom<&[u8]> for Value {
    type Error = KeyValueError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Value::new(value)
    }
}

impl<const N: usize> TryFrom<&[u8; N]> for Value {
    type Error = KeyValueError;

    fn try_from(value: &[u8; N]) -> Result<Self, Self::Error> {
        Value::new(*value)
    }
}

impl<'a> TryFrom<&'a str> for Value {
    type Error = KeyValueError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Value::new(value.as_bytes())
    }
}

pub struct KeyValue {
    pub key: Key,
    pub value: Value,
}

impl<K, V> TryFrom<(K, V)> for KeyValue
where
    K: TryInto<Key>,
    KeyValueError: From<<K as TryInto<Key>>::Error>,
    V: TryInto<Value>,
    KeyValueError: From<<V as TryInto<Value>>::Error>,
{
    type Error = KeyValueError;
    fn try_from((key, value): (K, V)) -> Result<Self, Self::Error> {
        Ok(KeyValue {
            key: key.try_into()?,
            value: value.try_into()?,
        })
    }
}

use std::fmt;

/// Convenient wrapper around `std::Result`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for blueberry-serde serialization/deserialization.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("does not support serde::Deserializer::deserialize_any")]
    DeserializeAnyNotSupported,

    #[error("expected 0 or 1 for bool, found {0}")]
    InvalidBoolEncoding(u8),

    #[error("invalid UTF-8 in string: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),

    #[error("invalid UTF-8 in string: {0}")]
    InvalidUtf8Owned(#[from] std::string::FromUtf8Error),

    #[error("number out of range")]
    NumberOutOfRange,

    #[error("sequences must have a known length")]
    SequenceMustHaveLength,

    #[error("unsupported type")]
    TypeNotSupported,

    #[error("unexpected end of input")]
    UnexpectedEof,

    #[error("invalid message header")]
    InvalidHeader,

    #[error("sequence index out of bounds: offset {0}")]
    SequenceIndexOutOfBounds(usize),
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Message(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Message(msg.to_string())
    }
}

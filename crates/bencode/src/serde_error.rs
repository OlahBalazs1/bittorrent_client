use serde::{de, ser};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("Syntax error")]
    Syntax,
    #[error("Unexpected eof")]
    Eof,
    #[error("Expected string")]
    ExpectedString,
    #[error("Expected integer")]
    ExpectedInt,
    #[error("Expected list")]
    ExpectedList,
    #[error("Expected map")]
    ExpectedMap,
    #[error("Error in string length")]
    StringLength,
    #[error("No terminator found for integer")]
    UnterminatedInt,
    #[error("Expected map end")]
    ExpectedMapEnd,
    #[error("Expected list end")]
    ExpectedListEnd,
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Message(msg.to_string())
    }
}
impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Message(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

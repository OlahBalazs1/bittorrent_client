use thiserror::Error;

#[derive(Debug, Error)]
#[error("{kind} (at position: {position})")]
pub struct BencodeError {
    position: usize,
    kind: BencodeErrorKind,
}
impl BencodeError {
    pub fn new(position: usize, kind: BencodeErrorKind) -> Self {
        Self { position, kind }
    }
}

#[derive(Debug, Error)]
pub enum BencodeErrorKind {
    #[error("No parentheses found for string.")]
    StringMissingParentheses,
    #[error("Dictionary key is invalid.")]
    DictionaryInvalidKey,
    #[error("In specifying string length.")]
    StringLength,
    #[error("Unexpected End of File.")]
    UnexpectedEOF,
    #[error("Unexpected character.")]
    UnexpectedCharacter,
    #[error("Unknown error.")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, BencodeError>;

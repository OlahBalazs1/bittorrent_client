#[derive(Debug)]
pub enum BencodeError {
    String(StringError),
    Dictionary(DictionaryError),
    Unknown,
}

#[derive(Debug)]
pub enum StringError {
    Length,
    MissingParentheses,
    Malformed,
}

#[derive(Debug)]
pub enum DictionaryError {
    InvalidKey,
}

pub type Result<T> = std::result::Result<T, BencodeError>;

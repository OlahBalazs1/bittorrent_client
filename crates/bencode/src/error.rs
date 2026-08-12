#[derive(Debug)]
pub enum BencodeError {
    String(StringError),
    Unknown,
}

#[derive(Debug)]
pub enum StringError {
    Length,
    MissingParentheses,
    Malformed,
}

pub type Result<T> = std::result::Result<T, BencodeError>;

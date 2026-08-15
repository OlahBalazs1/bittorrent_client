use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
};

use num::BigInt;

use crate::token::Token::Int;

#[derive(PartialEq, Eq, Clone)]
pub enum Token {
    String(Vec<u8>),
    Int(BigInt),
    List(Vec<Token>),
    Dictionary(HashMap<String, Token>),
}
impl Token {
    pub fn cast_string(self) -> Option<Vec<u8>> {
        self.try_into().ok()
    }
    pub fn cast_int(self) -> Option<BigInt> {
        self.try_into().ok()
    }
    pub fn cast_list(self) -> Option<Vec<Token>> {
        self.try_into().ok()
    }
    pub fn cast_dictionary(self) -> Option<HashMap<String, Token>> {
        self.try_into().ok()
    }
    pub fn cast_string_ref(&self) -> Option<&[u8]> {
        match self {
            Self::String(data) => Some(data),
            _ => None,
        }
    }

    pub fn cast_int_ref(&self) -> Option<&BigInt> {
        match self {
            Self::Int(data) => Some(data),
            _ => None,
        }
    }

    pub fn cast_list_ref(&self) -> Option<&Vec<Token>> {
        match self {
            Self::List(data) => Some(data),
            _ => None,
        }
    }

    pub fn cast_dictionary_ref(&self) -> Option<&HashMap<String, Token>> {
        match self {
            Self::Dictionary(data) => Some(data),
            _ => None,
        }
    }
    pub fn cast_string_mut(&mut self) -> Option<&mut [u8]> {
        match self {
            Self::String(data) => Some(data),
            _ => None,
        }
    }

    pub fn cast_int_mut(&mut self) -> Option<&mut BigInt> {
        match self {
            Self::Int(data) => Some(data),
            _ => None,
        }
    }

    pub fn cast_list_mut(&mut self) -> Option<&mut Vec<Token>> {
        match self {
            Self::List(data) => Some(data),
            _ => None,
        }
    }

    pub fn cast_dictionary_mut(&mut self) -> Option<&mut HashMap<String, Token>> {
        match self {
            Self::Dictionary(data) => Some(data),
            _ => None,
        }
    }

    pub fn bencode(self) -> Vec<u8> {
        let mut sink = Vec::<u8>::new();

        self.bencode_inner(&mut sink);

        sink
    }

    fn bencode_inner(self, sink: &mut Vec<u8>) {
        match self {
            Self::String(data) => {
                sink.extend_from_slice(format!("{}", data.len()).as_bytes());
                sink.push(b':');
                sink.extend_from_slice(&data);
            }
            Self::Int(data) => {
                sink.push(b'i');
                sink.extend_from_slice(format!("{data}").as_bytes());
                sink.push(b'e');
            }
            Self::List(data) => {
                sink.push(b'l');
                for t in data {
                    t.bencode_inner(sink);
                }
                sink.push(b'e');
            }
            Self::Dictionary(data) => {
                let mut sorted = data.into_iter().collect::<Vec<_>>();
                sorted.sort_by(|(a, _), (b, _)| a.cmp(b));

                sink.push(b'd');
                for (key, t) in sorted {
                    Token::String(key.into_bytes()).bencode_inner(sink);
                    t.bencode_inner(sink);
                }
                sink.push(b'e');
            }
        }
    }
}

#[derive(Debug)]
pub struct TokenConversionError;

impl Display for TokenConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token could not converted into the target type.")
    }
}

impl Error for TokenConversionError {}

impl TryFrom<Token> for Vec<u8> {
    type Error = TokenConversionError;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
        match token {
            Token::String(data) => Ok(data),
            _ => Err(TokenConversionError),
        }
    }
}

impl TryFrom<Token> for BigInt {
    type Error = TokenConversionError;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
        match token {
            Token::Int(data) => Ok(data),
            _ => Err(TokenConversionError),
        }
    }
}

impl TryFrom<Token> for Vec<Token> {
    type Error = TokenConversionError;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
        match token {
            Token::List(data) => Ok(data),
            _ => Err(TokenConversionError),
        }
    }
}

impl TryFrom<Token> for HashMap<String, Token> {
    type Error = TokenConversionError;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
        match token {
            Token::Dictionary(data) => Ok(data),
            _ => Err(TokenConversionError),
        }
    }
}

pub trait Tokenize {
    fn tokenize(self) -> Token;
}

impl Tokenize for Token {
    fn tokenize(self) -> Token {
        self
    }
}

impl Tokenize for BigInt {
    fn tokenize(self) -> Token {
        Int(self.into())
    }
}

impl Tokenize for String {
    fn tokenize(self) -> Token {
        Token::String(self.into())
    }
}

impl<T: Tokenize> Tokenize for Vec<T> {
    fn tokenize(self) -> Token {
        Token::List(self.into_iter().map(|e| e.tokenize()).collect())
    }
}

impl Tokenize for Vec<u8> {
    fn tokenize(self) -> Token {
        Token::String(self)
    }
}

impl<T: Tokenize> Tokenize for HashMap<String, T> {
    fn tokenize(self) -> Token {
        Token::Dictionary(
            self.into_iter()
                .map(|(key, val)| (key, val.tokenize()))
                .collect(),
        )
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(data) => write!(f, "{:?}", String::from_utf8_lossy(data))?,
            Self::Int(data) => <BigInt as Debug>::fmt(data, f)?,
            Self::List(data) => <Vec<_> as Debug>::fmt(data, f)?,
            Self::Dictionary(data) => <HashMap<_, _> as Debug>::fmt(data, f)?,
        }

        Ok(())
    }
}

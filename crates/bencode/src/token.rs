use std::{collections::HashMap, error::Error, fmt::Display};

use num::BigInt;

use crate::token::Token::Int;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Token {
    String(String),
    Int(BigInt),
    List(Vec<Token>),
    Dictionary(HashMap<String, Token>),
}
impl Token {
    pub fn cast_string(self) -> Option<String> {
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

    pub fn cast_string_ref(&self) -> Option<&str> {
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
    pub fn cast_string_mut(&mut self) -> Option<&mut str> {
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
}

#[derive(Debug)]
pub struct TokenConversionError;

impl Display for TokenConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token could not converted into the target type.")
    }
}

impl Error for TokenConversionError {}

impl TryFrom<Token> for String {
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

impl<T: Tokenize> Tokenize for HashMap<String, T> {
    fn tokenize(self) -> Token {
        Token::Dictionary(
            self.into_iter()
                .map(|(key, val)| (key, val.tokenize()))
                .collect(),
        )
    }
}

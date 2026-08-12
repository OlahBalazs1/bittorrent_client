use num::BigInt;
use std::{any::Any, collections::HashMap};

use crate::{
    error::{BencodeError, Result, StringError},
    token::Token,
};

pub fn parse_bencode(mut src: &str) -> Result<Token> {
    let mut out = None;
    while src.len() > 0 {
        #[cfg(test)]
        println!("{}", src.len());
        out = Some(call_parser(&mut src)?);
    }

    Ok(out.unwrap())
}

fn call_parser(src: &mut &str) -> Result<Token> {
    Ok(match src.chars().nth(0).unwrap() {
        '0'..='9' => Token::String(parse_string(src)?),
        'l' => {
            *src = &src[1..];
            Token::List(parse_list(src)?)
        }
        'd' => {
            *src = &src[1..];
            Token::Dictionary(parse_dictionary(src)?)
        }
        'i' => {
            *src = &src[1..];
            Token::Int(parse_integer(src)?)
        }
        _ => return Err(BencodeError::Unknown),
    })
}

fn parse_string(src: &mut &str) -> Result<String> {
    use StringError::*;
    if !matches!(
        src.bytes().nth(0).ok_or(BencodeError::String(Malformed))?,
        b'0'..=b'9'
    ) {
        return Err(BencodeError::String(Length));
    }
    let Some((length, rest)) = src.split_once(':') else {
        return Err(BencodeError::String(MissingParentheses));
    };

    let length_digit_count = length.len();

    let length = usize::from_str_radix(length, 10).map_err(|_| BencodeError::String(Length))?;

    let data = rest[0..length].to_owned();

    *src = &src[(length_digit_count + 1 + length)..];

    return Ok(data);
}

// DOESN'T COMPLY (but it doesn't really matter):
// - accepts leading zeroes
// - accepts negative zeroes
fn parse_integer(src: &mut &str) -> Result<BigInt> {
    let end = src
        .bytes()
        .position(|e| e == b'e')
        .ok_or(BencodeError::Unknown)?;

    let mut data = &src[0..end];
    if data.len() == 0 {
        return Err(BencodeError::Unknown);
    }
    let is_negative = data.as_bytes()[0] == b'-';
    if is_negative {
        data = &data[1..];
        if data.len() == 0 {
            return Err(BencodeError::Unknown);
        }
    }

    let num = BigInt::parse_bytes(data.as_bytes(), 10).ok_or(BencodeError::Unknown)?
        * if is_negative { -1 } else { 1 };

    *src = &src[(end + 1)..];

    return Ok(num);
}

fn parse_list(src: &mut &str) -> Result<Vec<Token>> {
    let mut list = vec![];
    while src.bytes().nth(0).ok_or(BencodeError::Unknown)? != b'e' {
        #[cfg(test)]
        println!("{src}");
        list.push(call_parser(src)?)
    }

    // get rid of the 'e'
    *src = &src[1..];

    Ok(list)
}

// DOESN'T COMPLY:
// - doesn't check if the keys are in sorted order
fn parse_dictionary(src: &mut &str) -> Result<HashMap<String, Token>> {
    let mut key: Option<String> = None;
    let mut dict = HashMap::new();
    while src.bytes().nth(0).ok_or(BencodeError::Unknown)? != b'e' {
        #[cfg(test)]
        println!("{src}");
        if key.is_none() {
            key = Some(parse_string(src)?);
        } else {
            dict.insert(key.take().unwrap(), call_parser(src)?);
        }
    }

    // get rid of the 'e'
    *src = &src[1..];

    Ok(dict)
}

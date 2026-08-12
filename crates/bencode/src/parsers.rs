use num::BigInt;
use std::{any::Any, collections::HashMap};

use crate::{
    error::{BencodeError, DictionaryError, Result, StringError},
    token::Token,
};

pub fn parse_bencode(mut src: &[u8]) -> Result<Token> {
    let mut out = None;
    while src.len() > 0 {
        #[cfg(test)]
        println!("{}", src.len());
        out = Some(call_parser(&mut src)?);
    }

    Ok(out.unwrap())
}

fn call_parser(src: &mut &[u8]) -> Result<Token> {
    Ok(match src.get(0).unwrap() {
        b'0'..=b'9' => Token::String(parse_string(src)?),
        b'l' => {
            *src = &src[1..];
            Token::List(parse_list(src)?)
        }
        b'd' => {
            *src = &src[1..];
            Token::Dictionary(parse_dictionary(src)?)
        }
        b'i' => {
            *src = &src[1..];
            Token::Int(parse_integer(src)?)
        }
        _ => return Err(BencodeError::Unknown),
    })
}

fn parse_string(src: &mut &[u8]) -> Result<Vec<u8>> {
    use StringError::*;
    if !matches!(
        src.get(0).ok_or(BencodeError::String(Malformed))?,
        b'0'..=b'9'
    ) {
        return Err(BencodeError::String(Length));
    }
    let Some(separator_index) = src.iter().position(|e| *e == b':') else {
        return Err(BencodeError::String(MissingParentheses));
    };
    let (length, rest) = (&src[..separator_index], &src[(separator_index + 1)..]);

    let length_digit_count = length.len();

    let length = usize::from_str_radix(
        &String::from_utf8(length.to_vec()).map_err(|_| BencodeError::String(Malformed))?,
        10,
    )
    .map_err(|_| BencodeError::String(Length))?;

    let data = rest[0..length].to_owned();

    *src = &src[(length_digit_count + 1 + length)..];

    return Ok(data);
}

// DOESN'T COMPLY (but it doesn't really matter):
// - accepts leading zeroes
// - accepts negative zeroes
fn parse_integer(src: &mut &[u8]) -> Result<BigInt> {
    let end = src
        .iter()
        .position(|e| *e == b'e')
        .ok_or(BencodeError::Unknown)?;

    let mut data = &src[0..end];
    if data.len() == 0 {
        return Err(BencodeError::Unknown);
    }
    let is_negative = data[0] == b'-';
    if is_negative {
        data = &data[1..];
        if data.len() == 0 {
            return Err(BencodeError::Unknown);
        }
    }

    let num = BigInt::parse_bytes(data, 10).ok_or(BencodeError::Unknown)?
        * if is_negative { -1 } else { 1 };

    *src = &src[(end + 1)..];

    return Ok(num);
}

fn parse_list(src: &mut &[u8]) -> Result<Vec<Token>> {
    let mut list = vec![];
    while *src.get(0).ok_or(BencodeError::Unknown)? != b'e' {
        list.push(call_parser(src)?)
    }

    // get rid of the 'e'
    *src = &src[1..];

    Ok(list)
}

// DOESN'T COMPLY:
// - doesn't check if the keys are in sorted order
fn parse_dictionary(src: &mut &[u8]) -> Result<HashMap<String, Token>> {
    use DictionaryError::*;
    let mut key: Option<String> = None;
    let mut is_parsing_key = true;
    let mut dict = HashMap::new();
    while *src.get(0).ok_or(BencodeError::Unknown)? != b'e' {
        if is_parsing_key {
            key = Some(
                parse_string(src)?
                    .try_into()
                    .map_err(|_| BencodeError::Dictionary(InvalidKey))?,
            );
            is_parsing_key = false;
        } else {
            if key.is_none() {
                return Err(BencodeError::Dictionary(InvalidKey));
            }
            is_parsing_key = true;
            dict.insert(key.take().unwrap(), call_parser(src)?);
        }
    }

    // get rid of the 'e'
    *src = &src[1..];

    Ok(dict)
}

use num::BigInt;
use std::collections::HashMap;

use crate::{
    Tokenize,
    error::{BencodeError, BencodeErrorKind::*, Result},
    token::Token,
};

pub fn parse_bencode(mut src: &[u8]) -> Result<Token> {
    // call_parser().1 is the amount of characters consumed
    // If the code got to this point without erroring, it's equal to src.len()
    Ok(call_parser(&mut src, 0)?.0)
}

fn call_parser(src: &mut &[u8], consumed: usize) -> Result<(Token, usize)> {
    Ok(
        match src
            .get(0)
            .ok_or(BencodeError::new(consumed, UnexpectedEOF))?
        {
            b'0'..=b'9' => tokenize_tuple(parse_string(src, consumed)?),
            b'l' => {
                *src = &src[1..];
                tokenize_tuple(parse_list(src, consumed)?)
            }
            b'd' => {
                *src = &src[1..];
                tokenize_tuple(parse_dictionary(src, consumed)?)
            }
            b'i' => {
                *src = &src[1..];
                tokenize_tuple(parse_integer(src, consumed)?)
            }
            _ => return Err(BencodeError::new(consumed, UnexpectedCharacter)),
        },
    )
}

fn parse_string(src: &mut &[u8], mut consumed: usize) -> Result<(Vec<u8>, usize)> {
    if !matches!(
        src.get(0)
            .ok_or(BencodeError::new(consumed, UnexpectedEOF))?,
        b'0'..=b'9'
    ) {
        return Err(BencodeError::new(consumed, StringLength));
    }
    let Some(separator_index) = src.iter().position(|e| *e == b':') else {
        return Err(BencodeError::new(consumed, StringMissingParentheses));
    };
    let (length, rest) = (&src[..separator_index], &src[(separator_index + 1)..]);

    let length_digit_count = length.len();

    let length = usize::from_str_radix(
        &String::from_utf8(length.to_vec())
            .map_err(|_| BencodeError::new(consumed, StringLength))?,
        10,
    )
    .map_err(|_| BencodeError::new(consumed, StringLength))?;

    let data = rest
        .get(0..length)
        .ok_or(BencodeError::new(rest.len() + consumed, UnexpectedEOF))?
        .to_owned();

    *src = &src[(length_digit_count + 1 + length)..];
    consumed += length_digit_count + 1 + length;

    return Ok((data, consumed));
}

// DOESN'T COMPLY (but it doesn't really matter):
// - accepts leading zeroes
// - accepts negative zeroes
fn parse_integer(src: &mut &[u8], mut consumed: usize) -> Result<(BigInt, usize)> {
    let end = src
        .iter()
        .position(|e| *e == b'e')
        .ok_or(BencodeError::new(consumed, Unknown))?;

    let mut data = &src[0..end];
    if data.len() == 0 {
        return Err(BencodeError::new(consumed, Unknown));
    }
    let is_negative = data[0] == b'-';
    if is_negative {
        data = &data[1..];
        if data.len() == 0 {
            return Err(BencodeError::new(consumed, Unknown));
        }
    }

    let num = BigInt::parse_bytes(data, 10).ok_or(BencodeError::new(consumed, Unknown))?
        * if is_negative { -1 } else { 1 };

    *src = &src[(end + 1)..];
    consumed += end + 1;

    return Ok((num, consumed));
}

fn parse_list(src: &mut &[u8], mut consumed: usize) -> Result<(Vec<Token>, usize)> {
    let mut list = vec![];
    while *src.get(0).ok_or(BencodeError::new(consumed, Unknown))? != b'e' {
        let (parsed, cnt) = call_parser(src, consumed)?;
        list.push(parsed);
        consumed += cnt;
    }

    // get rid of the 'e'
    *src = &src[1..];
    consumed += 1;

    Ok((list, consumed))
}

// DOESN'T COMPLY:
// - doesn't check if the keys are in sorted order
fn parse_dictionary(
    src: &mut &[u8],
    mut consumed: usize,
) -> Result<(HashMap<String, Token>, usize)> {
    let mut key: Option<String> = None;
    let mut is_parsing_key = true;
    let mut dict = HashMap::new();
    while *src
        .get(0)
        .ok_or(BencodeError::new(consumed, UnexpectedEOF))?
        != b'e'
    {
        if is_parsing_key {
            let (parsed, cnt) = parse_string(src, consumed)?;
            key = Some(
                parsed
                    .try_into()
                    .map_err(|_| BencodeError::new(consumed, DictionaryInvalidKey))?,
            );
            consumed += cnt;
            is_parsing_key = false;
        } else {
            if key.is_none() {
                return Err(BencodeError::new(consumed, DictionaryInvalidKey));
            }
            is_parsing_key = true;
            let (parsed, cnt) = call_parser(src, consumed)?;
            dict.insert(key.take().unwrap(), parsed);
            consumed += cnt;
        }
    }

    // get rid of the 'e'
    *src = &src[1..];
    consumed += 1;

    Ok((dict, consumed))
}

fn tokenize_tuple<T: Tokenize, U>(t: (T, U)) -> (Token, U) {
    let (t1, t2) = t;

    (t1.tokenize(), t2)
}

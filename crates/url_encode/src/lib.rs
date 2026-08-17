use std::fmt::Write;

pub fn url_encode(input: Vec<u8>) -> String {
    let mut out = String::new();
    for b in input {
        if should_be_encoded(b) {
            write!(out, "%{b:02X}").unwrap()
        } else {
            // guaranteed to be ASCII
            write!(out, "{}", b as char).unwrap()
        }
    }

    out
}

// I should somehow limit the input to ascii, however I'm the only one who'll use this so fuck it
pub fn url_decode(input: String) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut chars = input.chars();
    loop {
        let Some(i) = chars.next() else {
            break;
        };
        if !i.is_ascii() {
            return None;
        }
        match i {
            '%' => {
                let hex = format!("{}{}", chars.next()?, chars.next()?);
                out.push(u8::from_str_radix(&hex, 16).ok()?);
            }
            _ => out.push(i as u8),
        }
    }
    Some(out)
}

fn should_be_encoded(c: u8) -> bool {
    !matches!(c, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

#[cfg(test)]
mod tests {
    use crate::{url_decode, url_encode};

    #[test]
    fn encode() {
        let cases = [(
            "123űá/!+&%".as_bytes().to_vec(),
            "123%C5%B1%C3%A1%2F%21%2B%26%25".to_string(),
        )];

        for (raw, encoded) in cases {
            let test_encode = url_encode(raw);
            assert_eq!(test_encode, encoded);
        }
    }

    #[test]
    fn decode() {
        let cases = [(
            "123űá/!+&%".as_bytes().to_vec(),
            "123%C5%B1%C3%A1%2F%21%2B%26%25".to_string(),
        )];

        for (raw, encoded) in cases {
            let decode = url_decode(encoded).expect("Test value should be decodeable");
            assert_eq!(decode, raw);
        }
    }
}

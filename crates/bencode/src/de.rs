use crate::error::{Error, Result};
use Error::*;
use serde::{
    Deserialize,
    de::{self, SeqAccess},
    forward_to_deserialize_any,
};

pub struct Deserializer<'a> {
    input: &'a [u8],
}

struct BencodeList<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

struct BencodeMap<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

pub fn from_str<'a, T: Deserialize<'a>>(input: &'a str) -> Result<T> {
    let mut de: Deserializer = Deserializer::from_str(input);
    T::deserialize(&mut de)
}

pub fn from_bytes<'a, T: Deserialize<'a>>(input: &'a [u8]) -> Result<T> {
    let mut de: Deserializer = Deserializer::from_bytes(input);
    T::deserialize(&mut de)
}

impl<'a> Deserializer<'a> {
    pub fn from_str(input: &'a str) -> Self {
        Self::from_bytes(input.as_bytes())
    }

    pub fn from_bytes(input: &'a [u8]) -> Self {
        Self { input }
    }

    fn peek_char(&self) -> Result<u8> {
        self.input.get(0).copied().ok_or(Eof)
    }

    fn next_char(&mut self) -> Result<u8> {
        let val = self.peek_char()?;
        self.input = &self.input[1..];
        Ok(val)
    }

    fn parse_bytes(&mut self) -> Result<&'a [u8]> {
        if !matches!(self.peek_char()?, b'0'..=b'9') {
            return Err(ExpectedString);
        }
        let Some(separator_index) = self.input.iter().position(|e| *e == b':') else {
            return Err(Syntax);
        };
        let (length, rest) = (
            &self.input[..separator_index],
            &self.input[(separator_index + 1)..],
        );

        let length_digit_count = length.len();

        let length = usize::from_str_radix(
            &String::from_utf8(length.to_vec()).map_err(|_| StringLength)?,
            10,
        )
        .map_err(|_| StringLength)?;

        let data = rest.get(0..length).ok_or(Eof)?;

        self.input = &self.input[(length_digit_count + 1 + length)..];

        return Ok(data);
    }

    fn parse_integer(&mut self) -> Result<i64> {
        if self.next_char()? != b'i' {
            return Err(ExpectedInt);
        }
        let end = self
            .input
            .iter()
            .position(|e| *e == b'e')
            .ok_or(UnterminatedInt)?;

        let data = str::from_utf8(&self.input[0..end]).map_err(|_| Syntax)?;
        if data.len() == 0 {
            return Err(Syntax);
        }

        self.input = &self.input[(end + 1)..];

        Ok(data.parse().map_err(|_| Syntax)?)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.peek_char()? {
            b'0'..=b'9' => self.deserialize_bytes(visitor),
            b'i' => self.deserialize_i64(visitor),
            b'l' => self.deserialize_seq(visitor),
            b'd' => self.deserialize_map(visitor),
            _ => return Err(Syntax),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.parse_bytes()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i64(self.parse_integer()?)
    }
    fn deserialize_seq<V>(self, visitor: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.next_char()? != b'l' {
            return Err(ExpectedList);
        }
        let val = visitor.visit_seq(BencodeList { de: self })?;

        if self.next_char()? == b'e' {
            Ok(val)
        } else {
            Err(ExpectedListEnd)
        }
    }
    fn deserialize_map<V>(self, visitor: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.next_char()? != b'd' {
            return Err(ExpectedMap);
        }
        let val = visitor.visit_map(BencodeMap { de: self })?;

        if self.next_char()? == b'e' {
            Ok(val)
        } else {
            Err(ExpectedMapEnd)
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        byte_buf unit unit_struct newtype_struct tuple tuple_struct
        enum identifier ignored_any
    }
}

impl<'de> SeqAccess<'de> for BencodeList<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::prelude::v1::Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.de.peek_char()? == b'e' {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de> de::MapAccess<'de> for BencodeMap<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(
        &mut self,
        seed: K,
    ) -> std::prelude::v1::Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.de.peek_char()? == b'e' {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::de::{from_bytes, from_str};

    #[test]
    fn byte_string() {
        let case = [b'1', b':', 255];

        let good = [255];

        let decoded: &[u8] = from_bytes(&case).unwrap();

        assert_eq!(decoded, good)
    }

    #[test]
    fn string() {
        let case = b"3:abc";

        let decoded: &[u8] = from_bytes(case).unwrap();

        assert_eq!(decoded, b"abc")
    }

    #[test]
    fn integer() {
        let cases: [(&[_], i64); _] = [(b"i-32e", -32), (b"i42e", 42)];

        for (raw, good) in cases {
            let decoded: i64 = from_bytes(raw).unwrap();

            assert_eq!(decoded, good)
        }
    }

    #[test]
    fn list() {
        let cases: [(&str, Vec<i32>); _] = [("li-32ei42ee", vec![-32, 42])];

        for (raw, good) in cases {
            let decoded: Vec<i32> = from_str(raw).unwrap();

            assert_eq!(decoded, good)
        }
    }

    #[test]
    fn nested_list() {
        let (raw, good) = ("ll3:abcee", vec![vec!["abc"]]);
        let decoded: Vec<Vec<&str>> = from_str(raw).unwrap();

        assert_eq!(decoded, good)
    }

    #[test]
    fn struc() {
        #[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
        struct Test {
            a: String,
            b: i16,
            c: Vec<u32>,
            #[serde(rename = "with space")]
            with_space: i64,
        }

        let case = format!("d1:a0:1:bi3e1:cli1ee{}:with spacei-1ee", "with space".len());

        let good = Test {
            a: "".to_string(),
            b: 3,
            c: vec![1],
            with_space: -1,
        };

        let deserialized: Test = from_str(&case).unwrap();

        assert_eq!(deserialized, good)
    }
}

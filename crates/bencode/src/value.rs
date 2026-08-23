use std::collections::HashMap;

use serde::{Deserialize, Serialize, de::Visitor};
use serde_bytes::ByteBuf;

use crate::{Error::ValueExpected, Value::*};

struct ValueVisitor;

#[derive(PartialEq, Eq, Debug)]
pub enum Value {
    Str(Vec<u8>),
    Integer(i64),
    Map(HashMap<ByteBuf, Value>),
    List(Vec<Value>),
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Str(data) => serializer.serialize_bytes(data),
            Value::Integer(data) => data.serialize(serializer),
            Value::Map(data) => data.serialize(serializer),
            Value::List(data) => data.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a byte buf, an integer, a map or a list")
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Integer(v))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_i64(v as i64)
    }
    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Str(v))
    }
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_byte_buf(v.to_vec())
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_bytes(v.as_bytes())
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_byte_buf(v.into_bytes())
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut list = Vec::new();
        while let Some(elem) = seq.next_element()? {
            list.push(elem)
        }
        Ok(List(list))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut output: HashMap<_, Value> = HashMap::new();

        loop {
            let Some(key) = map.next_key::<ByteBuf>()? else {
                break;
            };

            let Some(val) = map.next_value()? else {
                break;
            };

            output.insert(key, val);
        }

        Ok(Map(output))
    }
}

#[cfg(test)]
mod tests {
    use crate::{from_bytes, to_bytes};

    use super::*;

    #[test]
    fn value_string() {
        let string = (b"3:abc", Str(b"abc".to_vec()));

        let val: Value = from_bytes(string.0).unwrap();

        assert_eq!(val, string.1)
    }

    #[test]
    fn value_string_deserialize() {
        let bencode = b"3:abc";
        let decoded: Value = from_bytes(bencode).unwrap();
        let encoded = to_bytes(&decoded).unwrap();

        assert_eq!(bencode as &[_], &encoded as &[_])
    }
    #[test]
    fn value_int_deserialize() {
        let bencode = b"i-3e";
        let decoded: Value = from_bytes(bencode).unwrap();
        let encoded = to_bytes(&decoded).unwrap();

        assert_eq!(bencode as &[_], &encoded as &[_])
    }
    #[test]
    fn value_list_deserialize() {
        let bencode = b"l3:abci-3ee";
        let decoded: Value = from_bytes(bencode).unwrap();
        let encoded = to_bytes(&decoded).unwrap();

        assert_eq!(bencode as &[_], &encoded as &[_])
    }
    #[test]
    fn value_dict_deserialize() {
        let bencode = b"d1:a1:be";
        let decoded: Value = from_bytes(bencode).unwrap();
        let encoded = to_bytes(&decoded).unwrap();

        assert_eq!(bencode as &[_], &encoded as &[_])
    }
}

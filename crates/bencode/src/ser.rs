use crate::{
    error::{
        Error::{self, UnsupportedType},
        Result,
    },
    from_bytes,
};
use serde::{
    Serialize,
    ser::{
        self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
};

pub struct Serializer {
    output: Vec<u8>,

    serializing_key: bool,
}

pub struct MapSerializer<'a> {
    serializer: &'a mut Serializer,
    serialized_keys: Vec<Vec<u8>>,
    serialized_values: Vec<Vec<u8>>,
}
impl<'a> MapSerializer<'a> {
    fn new(ser: &'a mut Serializer) -> Self {
        Self {
            serializer: ser,
            serialized_keys: Vec::new(),
            serialized_values: Vec::new(),
        }
    }
}

pub fn to_bytes<T: Serialize + ?Sized>(input: &T) -> Result<Vec<u8>> {
    let mut ser = Serializer::new();

    input.serialize(&mut ser)?;
    Ok(ser.output)
}
impl Serializer {
    fn new() -> Self {
        Self {
            output: Vec::new(),
            serializing_key: false,
        }
    }
}
impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeMap = MapSerializer<'a>;
    type SerializeSeq = Self;
    type SerializeStruct = MapSerializer<'a>;
    type SerializeStructVariant = MapSerializer<'a>;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;

    fn serialize_bytes(self, v: &[u8]) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.output
            .extend_from_slice(v.len().to_string().as_bytes());
        self.output.push(b':');
        self.output.extend_from_slice(v);
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.output.push(b'i');
        self.output.extend_from_slice(v.to_string().as_bytes());
        self.output.push(b'e');
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.output.push(b'i');
        self.output.extend_from_slice(v.to_string().as_bytes());
        self.output.push(b'e');
        Ok(())
    }
    fn serialize_seq(
        self,
        _len: Option<usize>,
    ) -> std::prelude::v1::Result<Self::SerializeSeq, Self::Error> {
        self.output.push(b'l');
        Ok(self)
    }
    fn serialize_map(
        self,
        _len: Option<usize>,
    ) -> std::prelude::v1::Result<Self::SerializeMap, Self::Error> {
        self.output.push(b'd');
        Ok(MapSerializer::new(self))
    }

    fn serialize_bool(self, v: bool) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_u64(if v { 1 } else { 0 })
    }
    fn serialize_u8(self, v: u8) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u16(self, v: u16) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u32(self, v: u32) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_i8(self, v: i8) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i16(self, v: i16) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i32(self, v: i32) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_f32(self, _v: f32) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        Err(UnsupportedType)
    }
    fn serialize_f64(self, _v: f64) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        Err(UnsupportedType)
    }

    fn serialize_char(self, v: char) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_str(self, v: &str) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.as_bytes())
    }
    fn serialize_none(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> std::prelude::v1::Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_some<T>(self, value: &T) -> std::prelude::v1::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    fn serialize_unit(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_unit_struct(
        self,
        _name: &'static str,
    ) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        variant.serialize(self)
    }
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> std::prelude::v1::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self)?;
        Ok(())
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> std::prelude::v1::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.output.push(b'd');
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)?;
        self.output.push(b'e');
        Ok(())
    }
    fn serialize_tuple(
        self,
        len: usize,
    ) -> std::prelude::v1::Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> std::prelude::v1::Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> std::prelude::v1::Result<Self::SerializeTupleVariant, Self::Error> {
        self.output.push(b'd');
        variant.serialize(&mut *self)?;
        self.output.push(b'd');
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> std::prelude::v1::Result<Self::SerializeStructVariant, Self::Error> {
        self.output.push(b'd');
        variant.serialize(&mut *self)?;
        self.output.push(b'd');
        Ok(MapSerializer::new(self))
    }
}

impl<'a> SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T>(&mut self, key: &T) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.serializing_key = true;
        let value = key.serialize(&mut **self)?;
        self.serializing_key = false;
        Ok(value)
    }
    fn serialize_value<T>(&mut self, value: &T) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }
    fn end(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.output.push(b'e');
        Ok(())
    }
}

impl<'a> SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T>(&mut self, value: &T) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.output.push(b'e');
        Ok(())
    }
}

impl<'a> SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(&mut *self, value)
    }

    fn end(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.output.push(b'e');
        Ok(())
    }
}

impl<'a> SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(&mut *self, value)
    }
    fn end(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.output.push(b'e');
        Ok(())
    }
}

impl<'a> SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(&mut *self, value)
    }

    fn end(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        self.output.push(b'e');
        Ok(())
    }
}

impl<'ser> SerializeMap for MapSerializer<'ser> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.serialized_keys.push(to_bytes(key)?);
        Ok(())
    }
    fn serialize_value<T>(&mut self, value: &T) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.serialized_values.push(to_bytes(value)?);
        Ok(())
    }
    fn end(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        let MapSerializer {
            serializer,
            serialized_keys,
            serialized_values,
        } = self;
        let mut key_val = std::iter::zip(serialized_keys, serialized_values).collect::<Vec<_>>();

        key_val.sort_by_cached_key(|(key, _)| from_bytes::<&[u8]>(key).unwrap().to_vec());
        for (mut key, mut val) in key_val {
            serializer.output.append(&mut key);
            serializer.output.append(&mut val);
        }

        serializer.output.push(b'e');
        Ok(())
    }
}

impl<'ser> SerializeStruct for MapSerializer<'ser> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeMap::serialize_key(&mut *self, key)?;
        SerializeMap::serialize_value(&mut *self, value)?;
        Ok(())
    }

    fn end(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        SerializeMap::end(self)
    }
}

impl<'ser> SerializeStructVariant for MapSerializer<'ser> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeMap::serialize_key(&mut *self, key)?;
        SerializeMap::serialize_value(&mut *self, value)?;
        Ok(())
    }
    fn end(self) -> std::prelude::v1::Result<Self::Ok, Self::Error> {
        SerializeMap::end(self)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::from_bytes;

    use super::*;
    #[test]
    fn fokin_everything() {
        #[derive(Deserialize, Serialize)]
        struct Test<'a> {
            a: i32,
            b: Vec<u32>,
            c: String,
            #[serde(borrow)]
            d: Test2<'a>,
        }
        #[derive(Deserialize, Serialize)]
        struct Test2<'a> {
            #[serde(with = "serde_bytes")]
            e: &'a [u8],
        }
        let data = b"d1:ai-3e1:bli3ee1:c3:abc1:dd1:e4:abcdee";

        let deserialized: Test = from_bytes(data).unwrap();

        let serialized = to_bytes(&deserialized).unwrap();

        assert_eq!(
            String::from_utf8_lossy(&serialized),
            String::from_utf8_lossy(data)
        )
    }
}

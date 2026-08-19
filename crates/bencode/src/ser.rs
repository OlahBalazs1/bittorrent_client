use crate::serde_error::{Error, Result};
use serde::{Serialize, ser};

pub struct Serializer {
    output: Vec<u8>,
}

pub fn to_bytes<T: Serialize>(input: &T) -> Result<Vec<u8>> {
    let mut ser = Serializer { output: Vec::new() };

    input.serialize(&mut ser)?;
    Ok(ser.output)
}
impl<'a> ser::Serializer for &'a mut Serializer {}

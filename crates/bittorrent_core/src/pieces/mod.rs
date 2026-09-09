mod request;
pub use request::*;

pub struct Block {
    pub piece: usize,
    pub begin: usize,
    pub data: Vec<u8>,
}

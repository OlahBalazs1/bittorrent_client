mod request;
pub use request::*;

#[derive(Debug, Clone)]
pub struct Block {
    pub piece: usize,
    pub begin: usize,
    pub data: Vec<u8>,
}

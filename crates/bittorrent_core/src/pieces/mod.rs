mod request;
pub use request::*;

#[derive(Debug, Clone)]
pub struct Block {
    pub piece: usize,
    pub begin: usize,
    pub data: Box<[u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockDescriptor {
    pub piece: usize,
    pub begin: usize,
    pub length: usize,
}

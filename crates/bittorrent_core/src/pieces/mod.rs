mod request;
pub use request::*;

pub struct IncomingBlock {
    pub piece: usize,
    pub begin: usize,
    pub data: Vec<u8>,
}

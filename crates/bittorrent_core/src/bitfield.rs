#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitfield {
    data: Box<[u8]>,
}

impl Bitfield {
    pub fn new(data: &[u8]) -> Self {
        Self { data: data.into() }
    }

    pub fn get(&self, piece: usize) -> bool {
        let index = piece / 8;
        let offset = piece % 8;

        self.data[index] & (1 << offset) != 0
    }

    pub fn set(&mut self, piece: usize, value: bool) {
        let index = piece / 8;
        let offset = piece % 8;

        if value {
            self.data[index] |= 1 << offset
        } else {
            self.data[index] &= !(1 << offset)
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

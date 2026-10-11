#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitfield {
    data: Box<[u64]>,
}

impl Bitfield {
    pub fn new(data: &[u8]) -> Self {
        let tail_start = data.len() / size_of::<u64>();
        let mut mucked: Vec<u64> = Vec::from(bytemuck::cast_slice(&data[0..tail_start]));
        let mut last: u64 = 0;
        for e in &data[tail_start..] {
            last |= *e as u64;
            last <<= 8;
        }
        mucked.push(last);
        Self {
            data: mucked.into(),
        }
    }

    pub fn get(&self, piece: usize) -> bool {
        let index = piece / 64;
        let offset = piece % 64;

        self.data[index] & (1 << offset) != 0
    }

    pub fn set(&mut self, piece: usize, value: bool) {
        let index = piece / 64;
        let offset = piece % 64;

        if value {
            self.data[index] |= 1 << offset
        } else {
            self.data[index] &= !(1 << offset)
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.data)
    }

    pub fn as_words(&self) -> &[u64] {
        &self.data
    }
}

#[derive(Debug)]
pub struct FloatRun {
    pub address: *const u8,
    pub values: Vec<f32>,
}

impl FloatRun {
    pub fn index_from_base(&self, base: *const u8) -> usize {
        (self.address as usize) - (base as usize)
    }
}

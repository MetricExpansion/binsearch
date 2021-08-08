#[derive(Debug)]
pub struct FloatRun<T> {
    pub address: *const u8,
    pub values: Vec<T>,
}

impl<T> FloatRun<T> {
    pub fn index_from_base(&self, base: *const u8) -> usize {
        (self.address as usize) - (base as usize)
    }
}

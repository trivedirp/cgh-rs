use crate::DigitalSource;

pub struct DigitalOr<const N: usize, T: DigitalSource> {
    pub sources: [T; N],
}

impl<const N: usize, T: DigitalSource> DigitalSource for DigitalOr<N, T> {
    fn eval(& mut self, i: usize) -> bool {
        self.sources.iter_mut().any(|source| source.eval(i))
    }
}
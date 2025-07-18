use std::iter::zip;

pub trait DigitalSource {
    fn eval(&mut self, i: usize) -> bool;
    fn chunk(&mut self, buf: &mut [bool], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = self.eval(i);
        }
    }
}
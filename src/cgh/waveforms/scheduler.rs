use crate::{do_clk, DigitalSource};
use std::iter::zip;

pub struct DigitalScheduler {
    pub source: Box<dyn DigitalSource + Send + Sync>,
    next_source: Option<(usize, Box<dyn DigitalSource + Send + Sync>)>,
}

impl DigitalScheduler {
    pub fn new<T: DigitalSource + Send + Sync + 'static>(source: T) -> Self {
        let source = Box::new(source);
        Self { source, next_source: None }
    }
    pub fn schedule<T: DigitalSource + Send + Sync + 'static>(&mut self, t_s: f64, source: T) {
        let source = Box::new(source);
        self.next_source = Some((do_clk(t_s), source));
    }
    #[inline]
    fn eval(&mut self, i: usize) -> bool {
        self.source.eval(i)
    }
    pub fn chunk(&mut self, buf: &mut [bool], start: usize) {
        if let Some((_, next_source)) = self.next_source.take_if(|(t_sclk, _)| start >= *t_sclk) {
            self.source = next_source;
        };
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = self.eval(i);
        }
    }
}

pub struct ConstDigital {
    value: bool,
}

impl ConstDigital {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl DigitalSource for ConstDigital {
    fn eval(& mut self, _i: usize) -> bool {
        self.value
    }
}
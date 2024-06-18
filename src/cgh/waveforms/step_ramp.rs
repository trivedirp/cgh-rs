use crate::{ao_clk, StepRampData};
use std::iter::zip;

#[derive(Debug)]
pub struct StepRamp {
    data: StepRampData,
    start_volts: f64,
    step_volts: f64,
    n_steps: usize,
    step_sclk: usize,
    offset_sclk: usize,
}

impl StepRamp {
    pub fn new(data: StepRampData) -> Self {
        let start_volts = data.start_volts;
        let step_volts = data.step_volts;
        let n_steps = ((data.end_volts - data.start_volts) / data.step_volts).round() as usize + 1;
        let step_sclk = ao_clk(data.step_s);
        let offset_sclk = ao_clk(data.offset_s);
        Self { data, start_volts, step_volts, n_steps, step_sclk, offset_sclk }
    }
    pub fn data(&self) -> &StepRampData { &self.data }
    pub fn n_steps(&self) -> usize { self.n_steps }
    #[inline]
    pub fn eval(&self, i: usize) -> f32 { (self.start_volts + self.step_volts * i.checked_sub(self.offset_sclk).map_or(0, |i| i / self.step_sclk % self.n_steps) as f64) as f32 }
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ { (0..).map(|i| self.eval(i)) }
    pub fn chunk(&self, buf: &mut [f32], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = self.eval(i);
        }
    }
}

#![allow(warnings)]
use crate::{ao_clk, SpiralData};
use std::iter::zip;
use std::f64::consts::PI;
#[derive(Debug)]
pub struct Spiral {
    data: SpiralData,
    start_volts: f64,
    range_volts: f64,
    period_sclk: usize,
    offset_sclk: usize,
    sine_on: bool,
}

impl Spiral {
    pub fn new(data: SpiralData) -> Self {
        let start_volts = data.start_volts;
        let range_volts = data.end_volts - data.start_volts;
        let period_sclk = ao_clk(data.period_s);
        let offset_sclk = ao_clk(data.offset_s);
        let sine_on = data.sine_on;
        Self { data, start_volts, range_volts, period_sclk, offset_sclk, sine_on }
    }
    pub fn data(&self) -> &SpiralData { &self.data }
    #[inline]
    pub fn eval_sin(&self, i: usize) -> f32 {
        let j = i.checked_sub(self.offset_sclk).map_or(0, |i| i);
        let t = { i.checked_sub(self.offset_sclk).map_or(0.0, |i| (i % self.period_sclk) as f64 / (self.period_sclk as f64) * f64::sin(6.0 * PI * (i % self.period_sclk) as f64 / (self.period_sclk as f64))) } ;
        (self.start_volts + self.range_volts * t) as f32
    } // sine
    pub fn eval_cos(&self, i: usize) -> f32 {
        let j = i.checked_sub(self.offset_sclk).map_or(0, |i| i);
        let t = { i.checked_sub(self.offset_sclk).map_or(0.0, |i| (i % self.period_sclk) as f64 / (self.period_sclk as f64) * f64::cos(6.0 * PI *(i % self.period_sclk) as f64 / (self.period_sclk as f64))) } ;
        (self.start_volts + self.range_volts * t) as f32
    } // cosine 
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ { (0..).map(|i| self.eval_sin(i)) }
    pub fn chunk(&self, buf: &mut [f32], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = if self.sine_on {self.eval_sin(i)} else {self.eval_cos(i)};
        }
    }
}

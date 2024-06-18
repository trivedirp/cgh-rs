use crate::{ao_clk, RampData};
use std::iter::zip;

#[derive(Debug)]
pub struct Ramp {
    data: RampData,
    start_volts: f64,
    range_volts: f64,
    period_sclk: usize,
    offset_sclk: usize,
    bidir_on: bool,
}

impl Ramp {
    pub fn new(data: RampData) -> Self {
        let start_volts = data.start_volts;
        let range_volts = data.end_volts - data.start_volts;
        let period_sclk = ao_clk(data.period_s);
        let offset_sclk = ao_clk(data.offset_s);
        let bidir_on = data.bidir_on;
        Self { data, start_volts, range_volts, period_sclk, offset_sclk, bidir_on }
    }
    pub fn data(&self) -> &RampData { &self.data }
    #[inline]
    pub fn eval_1(&self, i: usize) -> f32 {
        let t = i.checked_sub(self.offset_sclk).map_or(0.0, |i| 1.0 - f64::abs(2.0 * (i % self.period_sclk) as f64 / self.period_sclk as f64 - 1.0));
        (self.start_volts + self.range_volts * t) as f32
    } // symm triangular waveform
    pub fn eval_2(&self, i: usize) -> f32 {
        let j = i.checked_sub(self.offset_sclk).map_or(0, |i| i);
        let shoulder_t1 = ao_clk(0.5e-3) as f64* self.period_sclk as f64 / ao_clk(4.0e-3) as f64;
        let shoulder_t2 = ao_clk(2.0e-3) as f64* self.period_sclk as f64 / ao_clk(4.0e-3) as f64;
        let shoulder_t3 = ao_clk(2.5e-3) as f64* self.period_sclk as f64 / ao_clk(4.0e-3) as f64;
        //let shoulder_t1 = ao_clk(0.7e-3) as f64* self.period_sclk as f64 / ao_clk(4.0e-3) as f64;
        //let shoulder_t2 = ao_clk(1.8e-3) as f64* self.period_sclk as f64 / ao_clk(4.0e-3) as f64;
        //let shoulder_t3 = ao_clk(2.5e-3) as f64* self.period_sclk as f64 / ao_clk(4.0e-3) as f64;
        let mut t = 0.0;
        
        if (j % self.period_sclk) as f64 <= shoulder_t1 as f64 {
            t = i.checked_sub(self.offset_sclk).map_or(0.0, |i| 1.0 - f64::abs((i % self.period_sclk) as f64 / (shoulder_t1 as f64) - 1.0)); 
        } else if ((j % self.period_sclk) as f64 > shoulder_t1 as f64) && ((j % self.period_sclk) as f64 <= shoulder_t2 as f64) { 
            t = i.checked_sub(self.offset_sclk).map_or(0.0, |i| 1.0); 
        } else if ((j % self.period_sclk) as f64 > shoulder_t2 as f64) && ((j % self.period_sclk) as f64<= shoulder_t3 as f64) {
            t = i.checked_sub(self.offset_sclk).map_or(0.0, |i| 1.0 - f64::abs( ( (i % self.period_sclk) as f64 - shoulder_t2 as f64 ) / (shoulder_t1 as f64) ));
        } else {
            t = i.checked_sub(self.offset_sclk).map_or(0.0, |i| 0.0); 
        }
     
        (self.start_volts + self.range_volts * t) as f32
    } // bidirectional waveform
    pub fn eval_3(&self, i: usize) -> f32 {
        let j = i.checked_sub(self.offset_sclk).map_or(0, |i| i);
        let t = if (j % self.period_sclk) as f64 <= 0.8*self.period_sclk as f64 { i.checked_sub(self.offset_sclk).map_or(0.0, |i| 1.0 - f64::abs((i % self.period_sclk) as f64 / (0.8*self.period_sclk as f64) - 1.0)) } 
                else { i.checked_sub(self.offset_sclk).map_or(0.0, |i| 1.0 - f64::abs( ( (i % self.period_sclk) as f64 - 0.8*self.period_sclk as f64 ) / (0.2*self.period_sclk as f64) )) };
        (self.start_volts + self.range_volts * t) as f32
    } // sawtooth waveform 
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ { (0..).map(|i| self.eval_1(i)) }
    pub fn chunk(&self, buf: &mut [f32], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = if self.bidir_on {self.eval_2(i)} else {self.eval_1(i)};
        }
    }
}

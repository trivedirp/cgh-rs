use crate::{do_clk, PulseData};
use std::iter::zip;

#[derive(Debug)]
pub struct Pulse {
    data: PulseData,
    period_sclk: usize,
    on_sclk: usize,
    offset_sclk: usize,
}

impl Pulse {
    pub fn new(data: PulseData) -> Self {
        let period_sclk = do_clk(data.period_s);
        let on_sclk = do_clk(data.on_s);
        let offset_sclk = do_clk(data.offset_s);
        Self { data, period_sclk, on_sclk, offset_sclk }
    }
    pub fn data(&self) -> &PulseData { &self.data }
    #[inline]
    pub fn eval(&self, i: usize) -> bool { i.checked_sub(self.offset_sclk).map_or(false, |i| i % self.period_sclk < self.on_sclk) }
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ { (0..).map(|i| self.eval(i)) }
    pub fn chunk(&self, buf: &mut [bool], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = self.eval(i);
        }
    }
}


#[derive(Debug)]
pub struct PulseTrain {
    data: PulseData,
    period_sclk: usize,
    on_sclk: usize,
    offset_sclk: usize,
    led_pulse_train: Vec<f64>,
}

impl PulseTrain {
    pub fn new(data: PulseData, led_pulse_train: Vec<f64>) -> Self {
        let period_sclk = do_clk(data.period_s);
        let on_sclk = do_clk(data.on_s);
        let offset_sclk = do_clk(data.offset_s);
        Self { data, period_sclk, on_sclk, offset_sclk, led_pulse_train }
    }
    pub fn data(&self) -> &PulseData { &self.data }
    #[inline]
    pub fn eval(&self, i: usize) -> bool {
        // let on_s_train = vec![0.0, 1e-3, 10e-3, 100e-3, 1.0];
        let train_len = self.led_pulse_train.len();     
        let train_period_sclk = do_clk((train_len * self.period_sclk) as f64);
        let t = ( ((i % train_period_sclk) - (i % self.period_sclk)) / self.period_sclk ) % train_len ;
        let b = i.checked_sub(self.offset_sclk).map_or(false, |i| i % self.period_sclk < do_clk(self.led_pulse_train[t])) ;
        return b; 
    } 
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ { (0..).map(|i| self.eval(i)) }
    pub fn chunk(&self, buf: &mut [bool], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = self.eval(i);
        }
    }
}



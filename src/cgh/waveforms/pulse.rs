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
    pulse_duration_train: Vec<f64>,
    pulse_env: Vec<f64>,
    period_envclk: usize,
}

impl PulseTrain {
    pub fn new(data: PulseData, pulse_duration_train: Vec<f64>, pulse_env: Vec<f64>, period_env: f64) -> Self {
        let period_sclk = do_clk(data.period_s);
        let period_envclk = do_clk(period_env);
        let on_sclk = do_clk(data.on_s);
        let offset_sclk = do_clk(data.offset_s);
        Self { data, period_sclk, on_sclk, offset_sclk, pulse_duration_train, pulse_env, period_envclk }
    }
    pub fn data(&self) -> &PulseData { &self.data }
    #[inline]
    pub fn eval(&self, i: usize) -> bool {
        let subtrain_len = self.pulse_env.len();    
        let train_len = self.pulse_env.len() * self.pulse_duration_train.len();     
        let subtrain_period_sclk = do_clk((subtrain_len * self.period_envclk) as f64);
        let train_period_sclk = do_clk((train_len * self.period_envclk) as f64);
        
        let t = ( ((i % subtrain_period_sclk) - (i % self.period_envclk)) / self.period_envclk ) % subtrain_len ;
        let subtrain_no = ( ((i % train_period_sclk) - (i % subtrain_period_sclk)) / subtrain_period_sclk ) % self.pulse_duration_train.len() ;

        let b1 = i.checked_sub(0).map_or(false, |i| i % self.period_envclk < do_clk(self.pulse_env[t])) ;
        let b2 = i.checked_sub(self.offset_sclk).map_or(false, |i| i % self.period_sclk < do_clk(self.pulse_duration_train[subtrain_no]) ) ;
        let b = b1 & b2;
        return b; 
    }
    /* 
    pub fn eval(&self, i: usize) -> bool {
        let train_len = self.pulse_env.len();     
        let train_period_sclk = do_clk((train_len * self.period_envclk) as f64);
        let t = ( ((i % train_period_sclk) - (i % self.period_envclk)) / self.period_envclk ) % train_len ;
        let b1 = i.checked_sub(0).map_or(false, |i| i % self.period_envclk < do_clk(self.pulse_env[t])) ;
        let b2 = i.checked_sub(self.offset_sclk).map_or(false, |i| i % self.period_sclk < self.on_sclk) ;
        let b = b1 & b2;
        return b; 
    }
    */
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ { (0..).map(|i| self.eval(i)) }
    pub fn chunk(&self, buf: &mut [bool], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = self.eval(i);
        }
    }
}



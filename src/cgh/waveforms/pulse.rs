#![allow(warnings)]
use crate::{ao_clk, cgh::shared_state, do_clk, DigitalSource, DigitalOr, PulseData, SharedState};
use std::{iter::zip, sync::atomic::AtomicBool};

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
    pulse_duration_trainclk: (usize, usize, usize),
    pulse_envclk: (usize, usize, usize, usize),
    period_envclk: usize,
    pulse_env_period_sclk:usize,
    train_period_sclk:usize, 
    pulse_env_len: usize,
    pulse_duration_train_len: usize, 
}

impl PulseTrain {
    pub fn new(data: PulseData, pulse_duration_train: (f64, f64, f64), pulse_env: (f64, f64, f64, f64), period_env: f64) -> Self {
        let period_sclk = do_clk(data.period_s);
        let period_envclk = do_clk(period_env);
        let on_sclk = do_clk(data.on_s);
        let offset_sclk = do_clk(data.offset_s);
        let pulse_envclk = (do_clk(pulse_env.0), do_clk(pulse_env.1), do_clk(pulse_env.2), do_clk(pulse_env.3)); 
        let pulse_duration_trainclk = (do_clk(pulse_duration_train.0), do_clk(pulse_duration_train.1), do_clk(pulse_duration_train.2));

        let pulse_env_len = 4;  
        let pulse_duration_train_len = 3;
        let train_len =   pulse_env_len * pulse_duration_train_len;     
        // let pulse_env_len = self.pulse_env.len();    
        // let train_len = self.pulse_env.len() * self.pulse_duration_train.len();     
        let pulse_env_period_sclk = do_clk((pulse_env_len * period_envclk) as f64);
        let train_period_sclk = do_clk((train_len * period_envclk) as f64);

        Self { data, period_sclk, on_sclk, offset_sclk, pulse_duration_trainclk, pulse_envclk, period_envclk, pulse_env_period_sclk, train_period_sclk, pulse_env_len, pulse_duration_train_len }
    }
    pub fn data(&self) -> &PulseData { &self.data }
    #[inline]
    pub fn eval(&self, i: usize) -> bool {
        let t = ( ((i % self.pulse_env_period_sclk) - (i % self.period_envclk)) / self.period_envclk ) % self.pulse_env_len ;
        let subtrain_no = ( ((i % self.train_period_sclk) - (i % self.pulse_env_period_sclk)) / self.pulse_env_period_sclk ) % self.pulse_duration_train_len ;

        let b1 = i.checked_sub(0).map_or(false, |i| i % self.period_envclk < Self::get4(self.pulse_envclk, t)) ;
        let b2 = i.checked_sub(self.offset_sclk).map_or(false, |i| i % self.period_sclk < Self::get3(self.pulse_duration_trainclk, subtrain_no) ) ;
        b1 & b2
    } 
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ { (0..).map(|i| self.eval(i)) }
    pub fn chunk(&self, buf: &mut [bool], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = self.eval(i);
        }
    }
    fn get4(tuple: (usize, usize, usize, usize), i: usize) -> usize {
        match i {
            0 => tuple.0,
            1 => tuple.1,
            2 => tuple.2,
            3 => tuple.3,
            _ => panic!("Index out of range (must be 0..3)"),
        }
    }
    fn get3(tuple: (usize, usize, usize), i: usize) -> usize {
        match i {
            0 => tuple.0,
            1 => tuple.1,
            2 => tuple.2,
            _ => panic!("Index out of range (must be 0..2)"),
        }
    }
}

pub struct LivePulseTrain {
    // pulse_on_mask: bool,
    // experiment_save_start: bool,
    future_frame_index: usize,
    start_pulse_offsetclk: usize,
    data: PulseData,
    period_sclk: usize,
    on_sclk: usize,
    offset_sclk: usize,
    period_fast_plane_sclk: usize,
    pulse_duration_trainclk: (usize, usize, usize),
    // pulse_envclk: (usize, usize, usize, usize),
    pulse_envclk: usize,
    period_envclk: usize,
    pulse_env_period_sclk:usize,
    train_period_sclk:usize, 
    pulse_env_len: usize,
    pulse_duration_train_len: usize, 
}

impl LivePulseTrain {
    pub fn new(data: PulseData, pulse_duration_train: (f64, f64, f64), pulse_env: f64, period_env: f64, future_frame_index: usize) -> Self {
        // let pulse_on_mask = false;
        let start_pulse_offsetclk = 0;
        let period_fast_plane: f64 = 20.0e-3;
        let period_sclk = do_clk(data.period_s);
        let period_envclk = do_clk(period_env);
        let on_sclk = do_clk(data.on_s);
        let offset_sclk = do_clk(data.offset_s);
        let period_fast_plane_sclk = do_clk(period_fast_plane);
        // let pulse_envclk: (usize, usize, usize, usize) = (do_clk(pulse_env.0), do_clk(pulse_env.1), do_clk(pulse_env.2), do_clk(pulse_env.3)); 
        let pulse_envclk: usize = do_clk(pulse_env); 
        let pulse_duration_trainclk = (do_clk(pulse_duration_train.0), do_clk(pulse_duration_train.1), do_clk(pulse_duration_train.2));

        let pulse_env_len = 4;  
        let pulse_duration_train_len = 3;
        let train_len =   pulse_env_len * pulse_duration_train_len;     
        // let pulse_env_len = self.pulse_env.len();    
        // let train_len = self.pulse_env.len() * self.pulse_duration_train.len();     
        let pulse_env_period_sclk = do_clk((pulse_env_len * period_envclk) as f64);
        let train_period_sclk = do_clk((train_len * period_envclk) as f64);

        Self { // pulse_on_mask, 
            // experiment_save_start,
            future_frame_index,
            start_pulse_offsetclk,
            data, 
            period_sclk, 
            on_sclk, 
            offset_sclk, 
            period_fast_plane_sclk,
            pulse_duration_trainclk, 
            pulse_envclk, 
            period_envclk, 
            pulse_env_period_sclk, 
            train_period_sclk, 
            pulse_env_len, 
            pulse_duration_train_len }
    }
    pub fn iter(&mut self) -> impl Iterator<Item = bool> + '_ { (0..).map(|i| self.eval(i)) }

    fn get4(tuple: (usize, usize, usize, usize), i: usize) -> usize {
        match i {
            0 => tuple.0,
            1 => tuple.1,
            2 => tuple.2,
            3 => tuple.3,
            _ => panic!("Index out of range (must be 0..3)"),
        }
    }
    fn get3(tuple: (usize, usize, usize), i: usize) -> usize {
        match i {
            0 => tuple.0,
            1 => tuple.1,
            2 => tuple.2,
            _ => panic!("Index out of range (must be 0..2)"),
        }
    }
    pub fn data(&self) -> &PulseData { &self.data }
    /* pub fn update_pulse(&mut self, experiment_save_start: bool) {
        self.experiment_save_start = experiment_save_start;
    }*/
}

impl DigitalSource for LivePulseTrain {
    #[inline]
    fn eval(&mut self, i: usize) -> bool {
        let b = i.checked_sub(self.offset_sclk).map_or(false, |i| {
            if i == (self.future_frame_index * self.period_fast_plane_sclk ) {
                self.start_pulse_offsetclk = i;
                println!("start pulse offset {}",self.future_frame_index);
                /* if self.experiment_save_start {
                    self.start_pulse_offsetclk = i;
                    self.pulse_on_mask = true;
                    self.experiment_save_start = false;
                }*/
            } 

            let j = i.checked_sub(self.start_pulse_offsetclk).unwrap();
            let t = ( ((j % self.pulse_env_period_sclk) - (j % self.period_envclk)) / self.period_envclk ) % self.pulse_env_len ;
            let subtrain_no: usize = ( ((j % self.train_period_sclk) - (j % self.pulse_env_period_sclk)) / self.pulse_env_period_sclk ) % self.pulse_duration_train_len ;
            // let b1 = j % self.period_envclk < Self::get4(self.pulse_envclk, t);
            let b1 = j % self.period_envclk < self.pulse_envclk;
            // let b2 = j % self.period_sclk < Self::get3(self.pulse_duration_trainclk, subtrain_no);
            let b2 = j % self.period_sclk < self.pulse_duration_trainclk.2;
            // let b2 = j % self.period_sclk < self.pulse_duration_trainclk.1;
            b1 & b2
        });
        b
    } 
    /* pub fn eval(&self, i: usize) -> bool {
        let t = ( ((i % self.pulse_env_period_sclk) - (i % self.period_envclk)) / self.period_envclk ) % self.pulse_env_len ;
        let subtrain_no = ( ((i % self.train_period_sclk) - (i % self.pulse_env_period_sclk)) / self.pulse_env_period_sclk ) % self.pulse_duration_train_len ;

        let b1 = i.checked_sub(0).map_or(false, |i| i % self.period_envclk < self.pulse_envclk) ;
        let b2 = i.checked_sub(self.offset_sclk).map_or(false, |i| i % self.period_sclk < Self::get3(self.pulse_duration_trainclk, subtrain_no) ) ;
        b1 & b2
    } */
    fn chunk(&mut self, buf: &mut [bool], start: usize) {
        for (i, d) in zip(start..start + buf.len(), buf.iter_mut()) {
            *d = self.eval(i);
        }
    }
}


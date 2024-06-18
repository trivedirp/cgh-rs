use std::{
    io::Write,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex,
    },
};
use hdf5::{File, H5Type, Result};
use ndarray::{arr1, Array1, ArrayView};

pub fn vectoarr(v:&Vec<f32>) -> [f32; 9] {
    let s = v.as_slice();
    let arr: [f32;9] = match s.try_into() {
        Ok(a) => a,
        Err(..) => panic!("Expected a Vec of length {} but it was {}", 9, v.len()),
    };
    arr
}

pub struct SpimCalibration {
    pub volts_per_mm: f64,
    pub volts_at_0mm: f64,
}

impl SpimCalibration {
    pub fn new(volts_per_mm :f64, volts_at_0mm: f64) -> Self {
        Self { volts_per_mm, volts_at_0mm }
    }
    pub fn to_mm(&self, value_volts: f64) -> f64 {
        (value_volts - self.volts_at_0mm) / self.volts_per_mm
    }
    pub fn to_volts(&self, value_mm: f64) -> f64 {
        value_mm * self.volts_per_mm + self.volts_at_0mm
    }
    pub fn span_to_volts(&self, span_mm: f64) -> f64 {
        span_mm * self.volts_per_mm
    }
}

impl Default for SpimCalibration {
    fn default() -> Self {
        let volts_per_mm = -8.50;
        let volts_at_0mm = 0.750;
        Self { volts_per_mm, volts_at_0mm }
    }
}


pub struct SpimExptData {
    calib_coeff_filepath: PathBuf,
    vec_z_pos: Vec<f32>,
    vec_ao2: Vec<f32>,
    vec_ao3: Vec<f32>,
}

impl SpimExptData {
    pub fn new(calib_coeff_filepath: PathBuf) -> Self {
        Self {  calib_coeff_filepath,
                vec_z_pos: Vec::new(),
                vec_ao2: Vec::new(),
                vec_ao3: Vec::new(),
        }
    }
    pub fn read_calib_coeff(&self, port_id: &str) -> SpimCalibration {
        let file = File::open(self.calib_coeff_filepath.clone()).unwrap(); 
        let mut slope = 0.0f64 ;
        let mut intcpt = 0.0f64;
        if port_id == "front" {
            slope = file.dataset("slope_front").unwrap().read_scalar::<f64>().unwrap();
            intcpt = file.dataset("intercept_front").unwrap().read_scalar::<f64>().unwrap();
        } else if port_id == "side" {
            slope = file.dataset("slope_side").unwrap().read_scalar::<f64>().unwrap();
            intcpt = file.dataset("intercept_side").unwrap().read_scalar::<f64>().unwrap();
        }
        file.close().unwrap();
        SpimCalibration::new(slope, intcpt)
    }
    pub fn create_calib_file(&self) -> Result<()> {
        let _file = File::create("/data/rahul/data/spim_calib_data/calib_data.h5").unwrap(); 
        Ok(())
    }
    pub fn update_vec(&mut self, sample_z_position_mm: f32, ao2: f32, ao3: f32) -> Result<()> {
        self.vec_z_pos.push(sample_z_position_mm);
        self.vec_ao2.push(ao2);
        self.vec_ao3.push(ao3);
        Ok(())
    }
    pub fn write_calib_data(&self) -> Result<()> {
        assert_eq!(self.vec_z_pos.len(), self.vec_ao2.len());
        assert_eq!(self.vec_ao3.len(), self.vec_ao2.len());
        let file = File::append("/data/rahul/data/spim_calib_data/calib_data.h5").unwrap();
        // let group = file.create_group("calib").unwrap();
        let ds_z_pos = file.new_dataset::<f32>().shape([self.vec_z_pos.len()]).create("stage_z_mm").unwrap();
        let ds_ao2 = file.new_dataset::<f32>().shape([self.vec_ao2.len()]).create("ao2_side_v").unwrap();
        let ds_ao3 = file.new_dataset::<f32>().shape([self.vec_ao3.len()]).create("ao3_front_v").unwrap();
        ds_z_pos.write(&self.vec_z_pos).unwrap();
        ds_ao2.write(&self.vec_ao2).unwrap();
        ds_ao3.write(&self.vec_ao3).unwrap();
        file.close().unwrap();
        Ok(())
    }
    pub fn create_expt_params_file(&self, z_start_mm: f64, z_end_mm: f64, z_step_mm: f64, period_fast: f64, pulse_on_times: &Vec<f64>) -> Result<()> {
        let file = File::create("/data/rahul/data/spim_calib_data/expt_parsms.h5").unwrap(); 
        let ds_z_start_mm = file.new_dataset::<f64>().shape([1]).create("z_start_mm").unwrap();
        let ds_z_end_mm = file.new_dataset::<f64>().shape([1]).create("z_end_mm").unwrap();
        let ds_z_step_mm = file.new_dataset::<f64>().shape([1]).create("z_step_mm").unwrap();
        let ds_led_pulse_train = file.new_dataset::<f64>().shape([pulse_on_times.len()]).create("led_pulse_train").unwrap();
        ds_z_start_mm.write_scalar(&z_start_mm).unwrap();
        ds_z_end_mm.write_scalar(&z_end_mm).unwrap();
        ds_z_step_mm.write_scalar(&z_step_mm).unwrap();
        ds_led_pulse_train.write(&pulse_on_times).unwrap();
        file.close().unwrap();
        Ok(())
    }
}


pub struct CghPositions {
    pub clicks_x: Vec<f32>,
    pub clicks_y: Vec<f32>,
}

impl CghPositions {
    pub fn new() -> Self {
        Self {  clicks_x: Vec::new(),
                clicks_y: Vec::new(),
            }
    }
    pub fn update_vec(&mut self, pos_x: f32, pos_y: f32) -> Result<()> {
        self.clicks_x.push(pos_x);
        self.clicks_y.push(pos_y);
        Ok(())
    }
}

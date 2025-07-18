#![allow(warnings)]
use std::{
    convert::Into, fs::File, io::{BufReader, BufWriter, Read, Result, Write}, panic::{self, AssertUnwindSafe}, path::PathBuf, sync::{
        atomic::{AtomicBool, Ordering::Relaxed},Arc, Mutex, 
    }
};
use std::f32::consts::PI;
use crate::{cgh::hologram::fft_helper, CghMode, CuHelper, FftHelper, ZmqServer};
use cust::prelude::*;
use rand::random;
use gpu_utils::c32;

use super::zmq_server;
pub struct SLMConfig {
    pub slm_size: (u64, u64),
    N: usize,
    pub slm_bitdepth: i32,
    d_target_img: DeviceBuffer::<f32>, 
    d_phase_mask: DeviceBuffer::<u8>,
    stream: Arc<Stream>,
    cu_helper: Arc<CuHelper>,
    fft_helper: Arc<FftHelper>,
    zmq_server: Option<ZmqServer>,
}

impl SLMConfig {
    pub fn new(slm_size: (u64, u64), slm_bitdepth: i32, cgh_mode: CghMode) -> Self {
        let N = (slm_size.0 * slm_size.1) as usize;
        let d_target_img = DeviceBuffer::<f32>::zeroed(N).unwrap();
        let d_phase_mask = DeviceBuffer::<u8>::zeroed(N).unwrap();
        let server_on = cgh_mode == CghMode::CghInplane;
        // let server_on = cgh_mode == CghMode::CghInplane || cgh_mode == CghMode::SpimCalib;
        
        cust::init(cust::CudaFlags::empty()).unwrap();
        let device = Device::get_device(0).unwrap();
        let _ctx = Context::new(device).unwrap();
        let stream = Arc::new(Stream::new(StreamFlags::DEFAULT, None).unwrap());
        let cu_helper= Arc::new(CuHelper::new(slm_size, stream.clone()));
        let fft_helper= Arc::new(FftHelper::new(slm_size, stream.clone()));
            
        Self { slm_size,
            N,
            slm_bitdepth,
            d_target_img,
            d_phase_mask,
            stream,
            cu_helper,
            fft_helper,
            zmq_server: if server_on {
                Some(ZmqServer::new())
            } else {
                None
            }
        }
    }

    #[inline]
    /*
    pub fn read_target_img_file(&mut self, filepath: &PathBuf) -> Result<()> {
        let mut file = File::open(filepath).unwrap(); 
        let mut reader: BufReader<File> = BufReader::new(file);
        let mut buffer = vec!(u8::default(); self.target_img.elements() * 4);
        reader.read_exact(&mut buffer).unwrap();

        let mut target_img_vec = vec!(f32::default(); self.target_img.elements());
        target_img_vec = buffer.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
        self.target_img  = Array::new(&target_img_vec, self.dims);
        Ok(())
    }
    */
    pub fn read_phase_mask_file(& mut self, filepath: &PathBuf) -> Result<()> {
        let mut file = File::create(filepath).unwrap(); 
        let mut buffer = vec![u8::default(); self.N];
        file.read_exact(&mut buffer);
        self.d_phase_mask.copy_from(&buffer);
        Ok(())
    } 
    
    pub fn write_phase_mask_file(&mut self, filepath: &PathBuf) -> Result<()> {
        let mut buffer = vec![u8::default(); self.N];
        self.d_phase_mask.copy_to(&mut buffer);  
        // Write to file (as flat binary)
        let mut file = File::create("slm_ph_GS2D.bin").unwrap();
        for val in buffer.iter() {
            file.write_all(&val.to_le_bytes()).unwrap();
        } 

        match &self.zmq_server {
            Some(zmq_server) => {
                self.zmq_server.as_mut().expect("expected zmq server init").send_img(&buffer); 
            },
            None => {
                println!("zmq server not on");
            },      
        };
        Ok(())
    }

    pub fn send_pong(&mut self) -> Result<()> {
            match &self.zmq_server {
            Some(zmq_server) => {
                self.zmq_server.as_mut().expect("expected zmq server init").ping_pong(); 
            },
            None => {
                // println!("zmq server not on");
            },      
        };
        Ok(())
    }

    pub fn calc_gs2d(&mut self, n_iter:i32) {   
        let stream = self.stream.clone();  
        let slm_size_x = self.slm_size.0;
        let slm_size_y = self.slm_size.1;
        let slmc_x = slm_size_x as i32/2;
        let slmc_y = slm_size_y as i32/2;

        let dk:f32 = 1.0; // Assuming dk is defined somewhere in your context
        let beam_wid = 50.0; // Example value, adjust as necessary

        let mut d_slm_amp_ref = DeviceBuffer::<f32>::zeroed(self.N).unwrap();
        let mut d_slm_ph_init = DeviceBuffer::<f32>::zeroed(self.N).unwrap();
        let mut d_slm_field = DeviceBuffer::<c32>::zeroed(self.N).unwrap();
        let mut d_img_amp_ref = DeviceBuffer::<f32>::zeroed(self.N).unwrap();
        let mut d_img_ph_init = DeviceBuffer::<f32>::zeroed(self.N).unwrap();
        let mut d_img_field = DeviceBuffer::<c32>::zeroed(self.N).unwrap();

        let slm_amp_ref = vec![1.0; self.N]; // Gaussian, normalized, as Vec<f32>
        let slm_ph_init: Vec<f32> = (0..self.N)
            .map(|_| rand::random::<f32>()*2.0*PI)
            .collect(); 
        let img_amp_ref = vec![1.0; self.N];
        let img_ph_init = vec![0.0; self.N]; 

        d_slm_amp_ref.copy_from(&slm_amp_ref).unwrap();
        d_slm_ph_init.copy_from(&slm_ph_init).unwrap();
        d_img_amp_ref.copy_from(&img_amp_ref).unwrap();
        d_img_ph_init.copy_from(&img_ph_init).unwrap();

        self.cu_helper.float_to_cplx(&d_slm_amp_ref, &d_slm_ph_init, &d_slm_field);
        self.cu_helper.float_to_cplx(&d_img_amp_ref, &d_img_ph_init, &d_img_field);
        
        let mut d_slm_ph_calc = DeviceBuffer::<f32>::zeroed(self.N).unwrap();
        let mut d_img_ph_calc = DeviceBuffer::<f32>::zeroed(self.N).unwrap();
        let mut d_img_amp_calc = DeviceBuffer::<f32>::zeroed(self.N).unwrap();
        let mut d_slm_ph_calc_bin = DeviceBuffer::<u8>::zeroed(self.N).unwrap();

        let n_iter = 10;
        for _ in 0..n_iter {
            // (a) SLM field = shift(fft2(shift(img_field)))
            self.fft_helper.cfft(&d_slm_field, &d_img_field, 1 as i32);

            // (b) slm_ph_calc = arg(slm_field)
            self.cu_helper.get_arg(&d_slm_field, &d_slm_ph_calc);

            // (c) slm_field = arr_absargtocplx(slm_amp_ref, slm_ph_calc)
            self.cu_helper.abs_arg_to_cplx(&d_slm_amp_ref, &d_slm_ph_calc, &d_slm_field);

            // (d) img_field = shift(fft2(shift(slm_field)))
            self.fft_helper.cfft(&d_img_field, &d_slm_field, -1 as i32);

            // (e) img_ph_calc = arg(img_field)
            self.cu_helper.get_arg(&d_img_field, &d_img_ph_calc);

            // (f) img_amp_calc = abs(img_field)
            self.cu_helper.get_abs(&d_img_field, &d_img_amp_calc);

            // (g) img_field = arr_absargtocplx(img_amp_ref, img_ph_calc)
            self.cu_helper.abs_arg_to_cplx(&d_img_amp_ref, &d_img_ph_calc, &d_img_field);

        }

        self.cu_helper.binarize(&d_slm_ph_calc, &d_slm_ph_calc_bin);

        let mut output = vec![u8::default(); self.N];
        d_slm_ph_calc_bin.copy_to(&mut output).unwrap();  
        // Write to file (as flat binary)
        let mut file = File::create("slm_ph_GS2D.bin").unwrap();
        for val in output.iter() {
            file.write_all(&val.to_le_bytes()).unwrap();
        }      
    }

    pub fn calc_superpos3d(&mut self, shift_3d:(i32,i32,i32)) {     
        let shift_3d = (10, 10, 0);   
        let mut d_slm_ph_mod2pi = DeviceBuffer::<f32>::zeroed(self.N).unwrap();        
        let mut d_slm_ph_mod2pi_bin = DeviceBuffer::<u8>::zeroed(self.N).unwrap();        
        self.cu_helper.compute_slm_phase(&mut d_slm_ph_mod2pi, shift_3d);
        self.cu_helper.binarize(&d_slm_ph_mod2pi, &mut d_slm_ph_mod2pi_bin);

        let mut output = vec![u8::default(); self.N];
        d_slm_ph_mod2pi_bin.copy_to(&mut output).unwrap();  
        // Write to file (as flat binary)
        let mut file = File::create("slm_ph_superpos.bin").unwrap();
        for val in output.iter() {
            file.write_all(&val.to_le_bytes()).unwrap();
        }   
    }
}
    





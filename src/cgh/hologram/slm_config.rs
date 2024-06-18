use std::{
    convert::Into, fs::File, io::{BufReader, Read, Result, Write}, panic::{self, AssertUnwindSafe}, path::PathBuf, sync::{
        atomic::{AtomicBool, Ordering::Relaxed},Arc, Mutex, 
    }
};
use num_complex::Complex;
use arrayfire::*;
use std::f32::consts::PI;
use crate::{arr_absargtocplx, arr_floattocplx, binarize, rotate_xy, ZmqClient};
pub struct SLMConfig {
    slm_size: (u64, u64),
    slm_bitdepth: i32,
    dims: Dim4,
    target_img: Array<f32>, 
    phase_mask: Array<u8>,
    // zmq_client: ZmqClient,
}

impl SLMConfig {
    pub fn new(slm_size: (u64, u64), slm_bitdepth: i32) -> Self {
        Self { slm_size,
            slm_bitdepth,
            dims: Dim4::new(&[slm_size.0, slm_size.1, 1, 1]),
            target_img: constant(0.0, Dim4::new(&[slm_size.0, slm_size.1, 1, 1])),
            phase_mask: constant(0, Dim4::new(&[slm_size.0, slm_size.1, 1, 1])),
            // zmq_client: ZmqClient::new(),
        }
    }

    #[inline]
    pub fn read_target_img_file(&mut self, filepath: &PathBuf) -> Result<()> {
        let mut file = File::open(filepath).unwrap(); 
        let mut reader: BufReader<File> = BufReader::new(file);
        let mut buffer = vec!(u8::default(); self.target_img.elements() * 4);
        reader.read_exact(&mut buffer)?;

        let mut target_img_vec = vec!(f32::default(); self.target_img.elements());
        target_img_vec = buffer.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
        self.target_img  = Array::new(&target_img_vec, self.dims);
        Ok(())
    }

    pub fn read_phase_mask_file(& mut self, filepath: &PathBuf) -> Result<()> {
        let mut file = File::create(filepath).unwrap(); 
        let mut buffer = vec!(u8::default(); self.phase_mask.elements());
        file.read_exact(&mut buffer);
        self.phase_mask = Array::new(&buffer, self.dims);
        Ok(())
    } 
    
    pub fn write_phase_mask_file(&mut self, filepath: &PathBuf) -> Result<()> {
        let mut zmq_client = ZmqClient::new();
        let mut file = File::create(filepath).unwrap(); 
        let mut buffer = vec!(u8::default(); self.phase_mask.elements());
        zmq_client.send_img(&buffer); 
        self.phase_mask.host(&mut buffer);
        let _ = file.write_all(&mut buffer);
        Ok(())
    }

    pub fn calc_gs2d(&mut self, n_iter:i32) {
        // set_backend(Backend::CUDA); // Or Backend::OPENCL
        // set_device(0);          
        let img_size_x = self.slm_size.0;
        let img_size_y = self.slm_size.1; 
        let slm_size_x = self.slm_size.0;
        let slm_size_y = self.slm_size.1;
        let slmc_x = slm_size_x as i32/2;
        let slmc_y = slm_size_y as i32/2;
        let imgc_x= img_size_x as i32/2;
        let imgc_y= img_size_y as i32/2;

        let dk:f32 = 1.0; // Assuming dk is defined somewhere in your context
        let beam_wid = 50.0; // Example value, adjust as necessary

        let img_amp_ref = constant::<f32>(1.0, self.dims);  // Placeholder for actual target image
        let img_ph_init = randu::<f32>(self.dims) * 2.0 as f32*PI;
        let slm_amp_gauss = gaussian_kernel(slm_size_x.try_into().unwrap(), slm_size_y.try_into().unwrap(), beam_wid, beam_wid);
        let m = max_all(&slm_amp_gauss).0;
        let slm_amp_ref = slm_amp_gauss / m;
        let slm_ph_init = randu::<f32>(self.dims) * 2.0 as f32*PI;

        // let mut slm_field = cplx2(&slm_amp_ref, &slm_ph_init, false);
        // let mut img_field = cplx2(&img_amp_ref, &img_ph_init, false);
        let mut slm_field = arr_floattocplx(&slm_amp_ref, &slm_ph_init);
        let mut img_field = arr_floattocplx(&img_amp_ref, &img_ph_init);
    
        for _i in 0..n_iter {
            // slm_field = fft2(&img_field, 1.0, 0, 0);
            slm_field = shift( &fft2(&shift(&img_field, &[imgc_x,imgc_y,0,0]), 1.0, 0, 0), &[imgc_x,imgc_y,0,0] );
            let slm_ph_calc = arg(&slm_field);
            // discretize function needs to be implemented based on your discretization strategy
            // let slm_ph_calc_discr = discretize(&slm_ph_calc); // Placeholder for actual discretization function
            // slm_field = cplx2(&slm_amp_ref, &slm_ph_calc, false);
            slm_field = arr_absargtocplx(&slm_amp_ref, &slm_ph_calc);

            // img_field = fft2(&slm_field, 1.0,0, 0);
            img_field = shift( &fft2(&shift(&slm_field, &[slmc_x,slmc_y,0,0]), 1.0,0, 0), &[slmc_x,slmc_y,0,0] );
            let img_ph_calc = arg(&img_field);
            let img_amp_calc = abs(&img_field);
            // img_field = cplx2(&img_amp_ref, &img_ph_calc, false); // img E field for next iteration
            img_field = arr_absargtocplx(&img_amp_ref, &img_ph_calc); 
    
            // Error metrics calculation (adapt as necessary)
            // Note: Implement your own error metrics calculation based on the provided Julia code
        }
        self.phase_mask = binarize(&arg(&slm_field), self.slm_bitdepth);         
    }

    pub fn calc_superpos3d(&mut self, shift_3d:(i32,i32,i32)) {
        // set_backend(Backend::CUDA); // Or Backend::OPENCL
        // set_device(0);    
        let n_water = 1.33;
        let lambda = 1.0;    
        let k_total = 2.0*PI*n_water/lambda;  
        let img_size_x = self.slm_size.0;
        let img_size_y = self.slm_size.1; 
        let slm_size_x = self.slm_size.0;
        let slm_size_y = self.slm_size.1;
        let slmc_x = slm_size_x as i32/2;
        let slmc_y = slm_size_y as i32/2;
        let imgc_x= img_size_x as i32/2;
        let imgc_y= img_size_y as i32/2;
        // let dk:f32 = 1.0; 
        let dk:f32 = 2.0*PI/(img_size_x as f32*1.0); 
        let (x_shift, y_shift) = rotate_xy(shift_3d.0,shift_3d.1);
        let z_shift = shift_3d.2;
        let kx_shift = 2.0*PI/(x_shift as f32+1e-2);
        let ky_shift = 2.0*PI/(y_shift as f32+1e-2);
        let pitch_x_pix = (kx_shift/dk).floor();
        let pitch_y_pix = (ky_shift/dk).floor();

        let mut k_tot_sq: Array<f32> = constant::<f32>(k_total*k_total, self.dims);  
        let mut k_z: Array<f32> = constant::<f32>(0.0, self.dims);  
        let mut pitch_x: Array<f32> = constant::<f32>(pitch_x_pix, self.dims);  
        let mut pitch_y: Array<f32> = constant::<f32>(pitch_y_pix, self.dims);  
        let mut slm_ph = constant::<f32>(0.0, self.dims);

        let vec_pix_x: Vec<i32> = (1..=img_size_x as i32).collect();
        let vec_pix_y: Vec<i32> = (1..=img_size_y as i32).collect();
        let dims_x = Dim4::new(&[img_size_x, 1, 1, 1]);
        let dims_y = Dim4::new(&[1, img_size_y, 1, 1]);
        let arr_x = Array::new(&vec_pix_x, dims_x);
        let arr_y = Array::new(&vec_pix_y, dims_y);
        let pix_x = tile(&arr_x, dims_y);
        let pix_y = tile(&arr_y, dims_x);
        let a_slmc_x: Array<i32> = constant::<i32>(slmc_x, self.dims);  
        let a_slmc_y: Array<i32> = constant::<i32>(slmc_y, self.dims); 
        
        let k_x = sub(&pix_x, &a_slmc_x, true) * dk;
        let k_y = sub(&pix_y, &a_slmc_y,true) * dk;
        let k_xy_sq = add(&mul(&k_x, &k_x,true), &mul(&k_y,&k_y,true), true);
        let k_z = sqrt( &sub(&k_tot_sq, &k_xy_sq, true) );
        let phz = z_shift*k_z;
        let phx: Array<f32> = modulo(&pix_x, &pitch_x, true) * 2.0 as f32 * PI / pitch_x_pix;
        let phy: Array<f32> = modulo(&pix_y, &pitch_y, true) * 2.0 as f32 * PI / pitch_y_pix;

        slm_ph = add(&add(&phx, &phy, true), &phz, true);

        self.phase_mask = binarize(&slm_ph, self.slm_bitdepth); 
    }
}
    





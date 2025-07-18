use super::*;
use cust::memory::{AsyncCopyDestination, DeviceBox, DeviceCopy};
use cust::prelude::*;
use gpu_utils::{c32, f32i32};
use std::sync::Arc;
use std::{io::Write, mem::MaybeUninit};
use std::f32::consts::PI;


fn maybe_sync(stream: &Stream) {
    if cfg!(feature = "sync_check") {
        stream.synchronize().unwrap();
    }
}

pub struct CuHelper {
    pub slm_size: (u64, u64),
    N: usize,
    pub stream: Arc<Stream>,
    module: Module,
    grid_dim: (u32, u32, u32),
    block_dim: (u32, u32, u32),
}

impl CuHelper {
    pub fn new(slm_size: (u64, u64), stream: Arc<Stream>) -> Self {
        let N = (slm_size.0 * slm_size.1) as usize;
        let ptx = std::fs::read_to_string("src/cgh/cuda_c/cuda_utils.ptx").unwrap();
        let module = Module::from_ptx(ptx, &[]).unwrap();
        
        let block_dim = (16, 16, 1);
        let grid_dim = (
            (slm_size.0 as u32 + block_dim.0 - 1) / block_dim.0,
            (slm_size.1 as u32 + block_dim.1 - 1) / block_dim.1,
            1,
        );
        Self {
            slm_size,
            N,
            stream,
            module,
            grid_dim,
            block_dim,
        }
    }
    pub fn float_to_cplx(&self, d_slm_amp_ref: &DeviceBuffer<f32>, d_slm_ph_init: &DeviceBuffer<f32>, d_slm_field: &DeviceBuffer<c32>) -> Result<(), cust::error::CudaError> {
        let stream = self.stream.clone();
        let grid_dim =self.grid_dim;
        let block_dim = self.block_dim;
        let func = self.module.get_function("floattocplx")?;
        unsafe {
        launch!(
            func<<<grid_dim, block_dim, 0, stream>>>(
                d_slm_amp_ref.as_device_ptr(),
                d_slm_ph_init.as_device_ptr(),
                d_slm_field.as_device_ptr(),
                self.N as i32
            )
        )?;
        }
        Ok(())
    }
  
    pub fn abs_arg_to_cplx(&self, d_slm_amp_ref: &DeviceBuffer<f32>, d_slm_ph_calc: &DeviceBuffer<f32>, d_slm_field: &DeviceBuffer<c32>) -> Result<(), cust::error::CudaError> {
        let stream = self.stream.clone();
        let grid_dim =self.grid_dim;
        let block_dim = self.block_dim;
        let func = self.module.get_function("absargtocplx")?;
            unsafe {
                launch!(
                    func<<<grid_dim, block_dim, 0, stream>>>(
                        d_slm_amp_ref.as_device_ptr(),   
                        d_slm_ph_calc.as_device_ptr(),  
                        d_slm_field.as_device_ptr(),
                        self.N as i32
                    )
                )?;
            }
        
        Ok(())
    }

    pub fn get_arg(&self, d_slm_field: &DeviceBuffer<c32>, d_slm_ph_calc: &DeviceBuffer<f32>) -> Result<(), cust::error::CudaError> {
        let stream = self.stream.clone();
        let grid_dim =self.grid_dim;
        let block_dim = self.block_dim;
        let func = self.module.get_function("get_arg").unwrap();
            unsafe {
                launch!(
                    func<<<grid_dim, block_dim, 0, stream>>>(
                        d_slm_field.as_device_ptr(),   
                        d_slm_ph_calc.as_device_ptr(),  
                        self.N as i32
                    )
                )?;
            }
        Ok(())
    }

    pub fn get_abs(&self, d_img_field: &DeviceBuffer<c32>, d_img_amp_calc: &DeviceBuffer<f32>) -> Result<(), cust::error::CudaError> {
        let stream = self.stream.clone();
        let grid_dim =self.grid_dim;
        let block_dim = self.block_dim;        
        let func = self.module.get_function("get_abs")?;
            unsafe {
                launch!(
                    func<<<grid_dim, block_dim, 0, stream>>>(
                        d_img_field.as_device_ptr(),   
                        d_img_amp_calc.as_device_ptr(),  
                        self.N as i32
                    )
                )?;
            }
        Ok(())
    }
    
    pub fn compute_slm_phase(&self, d_slm_ph_mod2pi: &DeviceBuffer<f32>, (x_shift, y_shift, z_shift): (i32,i32,i32)) -> Result<(), cust::error::CudaError> {
        let stream = self.stream.clone();
        let grid_dim =self.grid_dim;
        let block_dim = self.block_dim;  

        let n_water = 1.33;
        let lambda = 1.030;    
        let k_total = 2.0*PI*n_water/lambda;  
        let k_tot_sq = k_total * k_total;
        let slm_wid = self.slm_size.0 as i32;
        let slm_ht = self.slm_size.1 as i32;
        let slmc_x = slm_wid as i32/2;
        let slmc_y = slm_ht as i32/2;
        let dk: f32 = 2.0*PI/(slm_wid as f32); 

        // let (x_shift, y_shift) = rotate_xy(x_shift,shify_shift);
        let kx_shift = 2.0*PI/(x_shift as f32+1e-2);
        let ky_shift = 2.0*PI/(y_shift as f32+1e-2);
        let pitch_x_pix = (kx_shift/dk).floor();
        let pitch_y_pix = (ky_shift/dk).floor();

        let func = self.module.get_function("compute_slm_phase")?;
        unsafe {
            launch!(
                func<<<grid_dim, block_dim, 0, stream>>>(
                    d_slm_ph_mod2pi.as_device_ptr(),
                    slm_wid,
                    slm_ht,
                    slmc_x,
                    slmc_y,
                    dk,
                    k_tot_sq,
                    z_shift,
                    pitch_x_pix,
                    pitch_y_pix,
                )
            )?;
        }
        Ok(())
    }

    pub fn binarize(&self, d_slm_ph_calc: &DeviceBuffer<f32>, d_slm_ph_calc_bin: &DeviceBuffer<u8>) -> Result<(), cust::error::CudaError> {
        let stream = self.stream.clone();
        let grid_dim =self.grid_dim;
        let block_dim = self.block_dim;
        let func = self.module.get_function("binarize").unwrap();
        
        unsafe {
            launch!(
                func<<<grid_dim, block_dim, 0, stream>>>(
                    d_slm_ph_calc.as_device_ptr(),
                    d_slm_ph_calc_bin.as_device_ptr(),
                    self.N as i32,
                    256
                )
            )?;
        }
        Ok(())
    }

    /*
    pub fn rotate_masked(&self, d_img: &DeviceSlice<i16>, d_mask: &DeviceSlice<u8>, d_img_rotated: &DeviceSlice<f32>, center: (i16, i16), theta_rad: f32, heading_rad: f32, mask_caudal_px: f32) {
        assert_eq!(d_img.len(), self.roi_n_pixels);
        assert_eq!(d_mask.len(), self.roi_n_pixels);
        assert_eq!(d_img_rotated.len(), self.roi_n_pixels);
        let module = &self.fish_tracker_mod;
        let stream = &self.stream;
        let (w, h): (u32, u32) = (self.roi_size.0.try_into().unwrap(), self.roi_size.1.try_into().unwrap());
        let sincos_theta = theta_rad.sin_cos();
        let sincos_heading = heading_rad.sin_cos();
        unsafe { launch!(module.rotate_masked<<<h, w, 0, stream>>>(d_img.as_device_ptr(), d_mask.as_device_ptr(), d_img_rotated.as_device_ptr(), center, sincos_theta, sincos_heading, mask_caudal_px)).unwrap() };
        maybe_sync(stream);
    }
    pub fn conj(&self, d_ref_fft: &DeviceSlice<c32>) {
        assert_eq!(d_ref_fft.len(), self.roi_n_pixels_fft);
        let module = &self.fish_tracker_mod;
        let stream = &self.stream;
        let (w, h): (u32, u32) = (self.roi_size.0.try_into().unwrap(), self.roi_size.1.try_into().unwrap());
        let w_div2plus1 = w / 2 + 1;
        let h_div2 = h / 2;
        unsafe { launch!(module.conj_batch<<<w_div2plus1, h_div2, 0, stream>>>(d_ref_fft.as_device_ptr())).unwrap() };
        maybe_sync(&self.stream);
    }
    pub fn normalize_masked(&self, d_img: &DeviceSlice<f32>, d_mask: &DeviceSlice<u8>, v_sub: f32, v_div: f32) {
        assert_eq!(d_img.len(), self.roi_n_pixels);
        let module = &self.fish_tracker_mod;
        let stream = &self.stream;
        let (w, h): (u32, u32) = (self.roi_size.0.try_into().unwrap(), self.roi_size.1.try_into().unwrap());
        let w_times_h_div_1024 = w * h / 1024;
        let ab = (1.0 / v_div, -v_sub / v_div);
        unsafe { launch!(module.normalize_masked<<<w_times_h_div_1024, 1024, 0, stream>>>(d_img.as_device_ptr(), d_mask.as_device_ptr(), ab)).unwrap() };
        maybe_sync(stream);
    }
    pub fn conv(&self, d_img_fft: &DeviceSlice<c32>, d_kernel_fft: &DeviceSlice<c32>, d_conv_fft: &DeviceSlice<c32>) {
        assert_eq!(d_img_fft.len(), self.roi_n_pixels_fft);
        assert_eq!(d_kernel_fft.len(), self.roi_n_pixels_fft);
        let module = &self.fish_tracker_mod;
        let stream = &self.stream;
        let (w, h): (u32, u32) = (self.roi_size.0.try_into().unwrap(), self.roi_size.1.try_into().unwrap());
        let w_div2plus1 = w / 2 + 1;
        let h_div2 = h / 2;
        unsafe { launch!(module.conv<<<w_div2plus1, h_div2, 0, stream>>>(d_img_fft.as_device_ptr(), d_kernel_fft.as_device_ptr(), d_conv_fft.as_device_ptr())).unwrap() };
        maybe_sync(stream);
    }
    
    pub fn inv_img_std(&self, d_img_local_mean: &DeviceSlice<f32>, d_img_sq_local_mean: &DeviceSlice<f32>, d_inv_img_std: &DeviceSlice<f32>) {
        assert_eq!(d_img_local_mean.len(), self.roi_n_pixels);
        assert_eq!(d_img_sq_local_mean.len(), self.roi_n_pixels);
        assert_eq!(d_inv_img_std.len(), self.roi_n_pixels);
        let module = &self.fish_tracker_mod;
        let stream = &self.stream;
        let (w, h): (u32, u32) = (self.roi_size.0.try_into().unwrap(), self.roi_size.1.try_into().unwrap());
        let w_div4 = w / 4;
        let k1024_div_wdiv4 = 1024 / w_div4;
        let w_div4_times_h_div_1024 = (w_div4 * h) / 1024;
        unsafe { launch!(module.img_std<<<w_div4_times_h_div_1024, (w_div4, k1024_div_wdiv4), 0, stream>>>(d_img_local_mean.as_device_ptr(), d_img_sq_local_mean.as_device_ptr(), d_inv_img_std.as_device_ptr())).unwrap() };
        maybe_sync(stream);
    }
    pub fn max_batch(&self, d_conv_xy: &DeviceSlice<f32>, d_penalty: &DeviceSlice<f32>, d_inv_img_std: &DeviceSlice<f32>, d_headings: &DeviceSlice<i16>, penalty_per_deg_and_heading_prior: (f32, i32), d_batch_data: &DeviceSlice<BatchData>) {
        assert_eq!(d_penalty.len(), self.roi_n_pixels);
        assert_eq!(d_inv_img_std.len(), self.roi_n_pixels);
        assert_eq!(d_headings.len(), d_batch_data.len());
        let n: u32 = d_headings.len().try_into().unwrap();
        let module = &self.fish_tracker_mod;
        let stream = &self.stream;
        match self.roi_size {
            (512, 512) => unsafe { launch!(module.max_batch_512x512<<<(32, n), 1024, 0, stream>>>(d_conv_xy.as_device_ptr(), d_inv_img_std.as_device_ptr(), d_penalty.as_device_ptr(), self.d_temp32_12.as_device_ptr())).unwrap() },
            (256, 256) => unsafe { launch!(module.max_batch_256x256<<<(32, n), 1024, 0, stream>>>(d_conv_xy.as_device_ptr(), d_inv_img_std.as_device_ptr(), d_penalty.as_device_ptr(), self.d_temp32_12.as_device_ptr())).unwrap() },
            _ => panic!("Unsupported image dimensions: {}, {}", self.roi_size.0, self.roi_size.1),
        }
        unsafe { launch!(module.max_batch2<<<n, 32, 0, stream>>>(d_headings.as_device_ptr(), penalty_per_deg_and_heading_prior, d_conv_xy.as_device_ptr(), d_inv_img_std.as_device_ptr(), self.d_temp32_12.as_device_ptr(), d_batch_data.as_device_ptr())).unwrap() };
        maybe_sync(stream);
    }  
    */
}
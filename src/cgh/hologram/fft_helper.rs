use cufft_sys::{cufftExecC2C, cufftHandle, cufftPlan2d, cufftPlanMany, cufftResult_t, cufftSetStream, cufftType};
use cust::prelude::*;
use std::sync::Arc;
use gpu_utils::c32;
use std::{mem::MaybeUninit, ptr::null_mut};

fn check(result: cufftResult_t) {
    assert_eq!(result, cufftResult_t::CUFFT_SUCCESS);
}

pub struct FftHelper {
    plan_c2c: cufftHandle,
}

impl FftHelper {
    pub fn new(size: (u64, u64), stream: Arc<Stream>) -> Self {
        let (width, height): (i32, i32) = (size.0.try_into().unwrap(), size.1.try_into().unwrap());
        // let mut fft_size = [height, width]; // outermost, innermost (contiguous)
        unsafe {
            let mut plan_c2c = MaybeUninit::<cufftHandle>::zeroed().assume_init();
            check(cufftPlan2d(&mut plan_c2c, width as _, height as _, cufft_sys::cufftType_t::CUFFT_C2C));
            check(cufftSetStream(plan_c2c, stream.as_inner() as _));
            Self {
                plan_c2c,
            }
        }
    }
    pub(crate) fn cfft(&self, d_img_fft: &DeviceSlice<c32>, d_img: &DeviceSlice<c32>, cuffft_dir: i32) {
        unsafe {
            check(cufftExecC2C(self.plan_c2c, d_img_fft.as_device_ptr().as_mut_ptr() as _, d_img.as_device_ptr().as_mut_ptr() as _, cuffft_dir));
        }
    }
}

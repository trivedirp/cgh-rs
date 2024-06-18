use std::{
    io::{Write, BufWriter},
    fs::File,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
};
use ndarray::{array, ArrayView};
use arrayfire::*;
use num_complex::Complex;
use std::f32::consts::PI;
use zmq::Context;
use std::thread::*;
use std::time::Duration;


pub fn arr_floattocplx(array_real:&Array<f32>, array_imag: &Array<f32>) -> Array<c32> {
    assert_eq!(array_real.elements(), array_imag.elements());
    let array_dims = array_real.dims();
    let mut vec_real = vec!(f32::default();array_real.elements());
    let mut vec_imag = vec!(f32::default();array_imag.elements());
    array_real.host(&mut vec_real);
    array_imag.host(&mut vec_imag);
    let mut vec_complex = vec!(c32::default();array_real.elements());
    vec_complex = vec_real.iter().zip(vec_imag.iter()).map(|(&vec_real, &vec_imag)| c32::new(vec_real, vec_imag) ).collect();
    let array_complex: Array<c32> = Array::new(&vec_complex, array_dims);
    return array_complex;
}

pub fn arr_absargtocplx(array_abs:&Array<f32>, array_arg: &Array<f32>) -> Array<c32> {
    assert_eq!(array_abs.elements(), array_arg.elements());
    let array_dims = array_abs.dims();
    let mut vec_abs = vec!(f32::default();array_abs.elements());
    let mut vec_arg = vec!(f32::default();array_arg.elements());
    array_abs.host(&mut vec_abs);
    array_arg.host(&mut vec_arg);
    let mut vec_complex = vec!(c32::default();array_abs.elements());
    vec_complex = vec_abs.iter().zip(vec_arg.iter()).map( |(&vec_abs, &vec_arg)| c32::new(vec_abs*f32::cos(vec_arg), vec_abs*f32::sin(vec_arg)) ).collect();
    let array_complex: Array<c32> = Array::new(&vec_complex, array_dims);
    return array_complex;
}

pub fn binarize(array_arg: &Array<f32>, bitdepth: i32) -> Array<u8> {
    let array_dims = array_arg.dims();
    let n_bins = (2 as i32).pow(bitdepth.try_into().unwrap());
    let array_disc = add(array_arg, &constant::<f32>(PI, array_dims), false) * (n_bins-1) as f32 / (2.0 as f32*PI); 
    let array_bin = floor(&array_disc).cast::<u8>();
    return array_bin;
}

pub fn rotate_xy(x: i32,y: i32) -> (i32,i32){
    let xy = array![x as f32, y as f32];
    let angle: f32 = 15.0 * PI / 180.0;
    let rotn = array![[f32::cos(angle), -1.0 as f32*f32::sin(angle)], [f32::sin(angle), f32::cos(angle)]];
    let rot_xy = rotn.dot(&xy);
    // println!("\nXY: {}\n", rot_xy);
    (rot_xy[0].floor() as i32, rot_xy[1].floor() as i32)
}

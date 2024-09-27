use crate::CghMode;
use crossbeam::atomic::AtomicCell;
use cust::memory::DeviceBuffer;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU16};
use std::sync::Arc;

pub struct SharedState {
    pub cgh_mode: CghMode,
    // pub zstage_msg: ZStageMessage,
    pub start_save: AtomicBool,
    pub save: AtomicBool,
    pub stop_save: AtomicBool,
    pub save_img: AtomicBool,
    pub led_pulse_on: AtomicBool,
    pub start_calib: AtomicBool,
    pub save_calib: AtomicBool,
    pub stop_calib: AtomicBool,
    pub save_expt: AtomicBool,
    pub v_max: AtomicCell<f32>,
    pub n_datapts: AtomicCell<u16>,
    pub d_roi: DeviceBuffer<u16>,
    pub ao0: AtomicCell<f32>,
    pub ao1: AtomicCell<f32>,
    pub ao2: AtomicCell<f32>,
    pub ao3: AtomicCell<f32>,
    pub save_divisor: AtomicI32,
    pub camera_temperature: AtomicCell<f32>,
    pub frame_rate: Arc<AtomicCell<f32>>,
    pub sample_z_home: AtomicBool,
    pub sample_z_on: AtomicBool,
    pub sample_z_off: AtomicBool,
    pub sample_z_manual_target_mm: AtomicCell<f32>,
    pub sample_z_event_status: AtomicU16,
    pub sample_z_position_mm: AtomicCell<f32>,
    pub shift_3d: AtomicCell<(i32,i32,i32)>,
    pub cgh_zero_ord: AtomicCell<(i32,i32)>,
    pub save_zero_ord: AtomicBool,
    pub enable_click_cgh: AtomicBool,
    pub generate_new_holo: AtomicBool,
}

impl SharedState {
    pub fn new(cgh_mode: CghMode, size: (usize, usize)) -> SharedState {
        let shared_state = SharedState {
            cgh_mode,
            start_save: AtomicBool::new(false),
            save: AtomicBool::new(false),
            stop_save: AtomicBool::new(false),
            save_img: AtomicBool::new(false),
            led_pulse_on: AtomicBool::new(false),
            start_calib: AtomicBool::new(false),
            save_calib: AtomicBool::new(false),
            stop_calib: AtomicBool::new(false),
            save_expt: AtomicBool::new(false),
            v_max: AtomicCell::new(10000.0f32),
            n_datapts: AtomicCell::new(0),
            d_roi: DeviceBuffer::<u16>::zeroed(size.0 * size.1).unwrap(),
            ao0: AtomicCell::new(0f32),
            ao1: AtomicCell::new(0f32),
            ao2: AtomicCell::new(0f32),
            ao3: AtomicCell::new(0f32),
            save_divisor: AtomicI32::new(1),
            camera_temperature: AtomicCell::new(f32::NAN),
            frame_rate: Arc::new(AtomicCell::new(f32::NAN)),
            sample_z_home: AtomicBool::new(false),
            sample_z_on: AtomicBool::new(false),
            sample_z_off: AtomicBool::new(false),
            sample_z_position_mm: AtomicCell::new(0f32),
            sample_z_manual_target_mm: AtomicCell::new(0.0f32),
            sample_z_event_status: AtomicU16::new(0u16),
            shift_3d: AtomicCell::new((0,0,0)),
            cgh_zero_ord: AtomicCell::new((0,0)),
            save_zero_ord: AtomicBool::new(false),
            enable_click_cgh: AtomicBool::new(false),
            generate_new_holo: AtomicBool::new(false),
        };
        shared_state
    }
}

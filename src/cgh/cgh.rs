use crate::{Pulse, PulseTrain, PulseData, Ramp, RampData, Spiral, SpiralData, CghMode, SLMConfig,
    SharedState, StepRamp, StepRampData, SpimCalibration, SpimExptData, AO_SAMPLE_CLK, DO_SAMPLE_CLK};
use async_writer::AsyncWriter;
use cust::{memory::AsyncCopyDestination, prelude::DeviceBuffer, stream::Stream};
use dcam::DcamCamera;
use ixxat::{PmdDevice, MotionProfile};
use log::{debug, info};
use nimhddk::XSeriesDevice;
use npp::StreamContext;
use npp_sys::cudaSetDevice;
use arrayfire::{set_device, set_backend, Backend};
use std::{
    io::Write,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex,
    },
    thread::{spawn, JoinHandle},
};

pub struct Cgh {
    thread: Option<JoinHandle<()>>,
    daq: Arc<XSeriesDevice>,
    camera: Arc<Mutex<DcamCamera>>,
    // calibration_front_488: SpimCalibration,
    calibration_side_488: SpimCalibration,
    // calibration_front_561: SpimCalibration,
    calibration_side_561: SpimCalibration,
    spim_data: Arc<Mutex<SpimExptData>>,
    pub size: (usize, usize),
    pub shared_state: Arc<SharedState>,
    pub stream: Arc<Stream>,
    pub stream_context: StreamContext,
    pub data_raw_path: Option<Arc<PathBuf>>,
    sample_z_stage: Arc<PmdDevice>,
    fl_path: Arc<PathBuf>,
    slm: Arc<Mutex<SLMConfig>>,
    slm_path: Arc<PathBuf>,
}

impl Cgh {
    pub fn new(cgh_mode: CghMode, stream: Arc<Stream>, fl_path: Arc<PathBuf>) -> Self {
        let thread = None;
        let stream_context = StreamContext::new(&stream);
        let velocity_mmps = 11.0;
        let acceleration_mmpsps = 600.0;
        let profile = MotionProfile::SCurve;
        let sample_z_stage = Arc::new(PmdDevice::new(velocity_mmps, acceleration_mmpsps, profile));
        // let daq = Arc::new(XSeriesDevice::new(10, 0).unwrap());
        let daq = Arc::new(XSeriesDevice::default());
        let mut camera = DcamCamera::new(0, None, false, 1000);
        camera.set_trigger_source(dcam::TriggerSource::External);
        camera.set_trigger_active(dcam::TriggerActive::SyncReadout);
        // camera.set_trigger_active(dcam::TriggerActive::Edge);
        camera.set_trigger_polarity(dcam::TriggerPolarity::Positive);
        camera.set_binning(2);
        // camera.set_exposure(0.00072);
        camera.set_exposure(0.00144);
        // camera.set_exposure(0.00598);
        // camera.set_trigger_global_exposure(dcam::TriggerGlobalExposure::GlobalReset);
        
        camera.properties.set_width(1024 * 2);
        camera.properties.set_height(512 * 2);
        camera.properties.set_offset_x(512 * 2);
        camera.properties.set_offset_y(256 * 2);
        
        let calibration_side_488 = SpimCalibration::new(0.0, 0.0);
        let calibration_side_561 = SpimCalibration::new(0.0, 0.0);
        let spim_data = Arc::new(Mutex::new(SpimExptData::new(PathBuf::from("/data/rahul/data/spim_calib_data/calib_coeff.h5"))));
        let size = (camera.width().try_into().unwrap(), camera.height().try_into().unwrap());
        let shared_state = Arc::new(SharedState::new(cgh_mode, size));
        let camera = Arc::new(Mutex::new(camera));

        let slm_size_x = 1920;
        let slm_size_y = 1152;
        let slm_size = (slm_size_x, slm_size_y);
        let slm_bitdepth = 8;
        let slm_path = fl_path.clone();
        let slm = Arc::new(Mutex::new(SLMConfig::new(slm_size, slm_bitdepth, cgh_mode)));

        Self {
            thread,
            daq,
            camera,
            // calibration_front_488,
            // calibration_front_561,
            calibration_side_488,
            calibration_side_561,
            spim_data,
            size,
            shared_state,
            stream,
            stream_context,
            data_raw_path: None,
            sample_z_stage,
            fl_path,
            slm,
            slm_path,
        }
    }
    pub fn start(&mut self, gpu_index: i32, stop: Arc<AtomicBool>, thread_panic: Arc<AtomicBool>) {
        debug!("start");
        let daq = self.daq.clone();
        assert_eq!(self.daq.DO().sample_clock(), DO_SAMPLE_CLK);
        assert_eq!(self.daq.AO().sample_clock(), AO_SAMPLE_CLK);

        let stack_mode = self.shared_state.cgh_mode == CghMode::Stack488 || self.shared_state.cgh_mode == CghMode::Stack561 || self.shared_state.cgh_mode == CghMode::Stack2ch;
        let z_start_mm: f64 = if stack_mode {150.0e-3} else {1.0e-3};
        let z_end_mm: f64 = if stack_mode {-150.0e-3} else {0.0e-3};
        
        let period_fast_3d = 20.0e-3;
        let period_fast_plane = 20.0e-3;
        let period_spiral = 1.0e-3;
        let period_slm = 100.0e-3;        
    
        let bidir_on = false;
        let period_fast = if self.shared_state.cgh_mode == CghMode::CghInplane {period_fast_plane} else {period_fast_3d};
        let pulse_period = 100e-3;
        let mut pulse_on_times: Vec<f64> = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.5e-3, 0.5e-3, 0.5e-3, 0.5e-3, 0.5e-3, 1e-3, 1e-3, 1e-3, 1e-3, 1e-3, 2e-3, 2e-3, 2e-3, 2e-3, 2e-3];
        // let mut pulse_on_times: Vec<f64> = vec![0.0, 0.0, 0.0, 0.0, 0.0, 1e-3, 1e-3, 1e-3, 1e-3, 1e-3, 5e-3, 5e-3, 5e-3, 5e-3, 5e-3];
        // let mut pulse_on_times: Vec<f64> = vec![0.0, 0.0, 0.0, 0.0, 0.0, 1e-3, 1e-3, 1e-3, 1e-3, 1e-3, 5e-3, 5e-3, 5e-3, 5e-3, 5e-3, 10e-3, 10e-3, 10e-3, 10e-3, 10e-3];
        pulse_on_times.extend(vec![0.0; 100]);
        let n_pulses = pulse_on_times.len();

        let steptime_slow_galvo = match self.shared_state.cgh_mode {
            // CghMode::CghInplane => n_pulses as f64*pulse_period,
            CghMode::Stack2ch => 2.0 * period_fast,
            _ => period_fast
        };
        // let mut fast_galvo_front_peak = 2.0; 
        let mut fast_galvo_side_peak = 10.0; 
        if self.shared_state.cgh_mode == CghMode::CghInplane {
            // fast_galvo_front_peak = 0.625;
            fast_galvo_side_peak = 5.0;       
            // fast_galvo_front_peak = 4.0;
            // fast_galvo_side_peak = 10.0;
        } 
        let spiral_amp = 0.0e-3; //voltage amplitude for spiral galvo 

        let t0 = period_fast * 0.05;
        let on_laser_s = match bidir_on { 
            false => period_fast * 0.4, // for trianglular waveform
            // false => period_fast * 0.64, // for sawtooth waveform 
            true => period_fast * 0.25
        };

        /* let fast_galvo_front = match bidir_on { 
            false => Ramp::new(RampData { period_s: period_fast, offset_s: t0 - 100e-6, start_volts: -1.0*fast_galvo_front_peak, end_volts: fast_galvo_front_peak, bidir_on }),
            true => Ramp::new(RampData { period_s: 2.0*period_fast, offset_s: t0 - 100e-6, start_volts: -1.0*fast_galvo_front_peak, end_volts: fast_galvo_front_peak, bidir_on })
        }; */
        let fast_galvo_side = match bidir_on {
            false => Ramp::new(RampData { period_s: period_fast, offset_s: t0 - 100e-6, start_volts: -1.0*fast_galvo_side_peak, end_volts: fast_galvo_side_peak, bidir_on }),
            true => Ramp::new(RampData { period_s: 2.0*period_fast, offset_s: t0 - 100e-6, start_volts: -1.0*fast_galvo_side_peak, end_volts: fast_galvo_side_peak, bidir_on })
        };

        let spiral_galvo_x = Spiral::new(SpiralData { period_s: period_spiral, offset_s: t0, start_volts: -1.0*spiral_amp, end_volts: spiral_amp, sine_on: true });
        let spiral_galvo_y = Spiral::new(SpiralData { period_s: period_spiral, offset_s: t0, start_volts: -1.0*spiral_amp, end_volts: spiral_amp, sine_on: false });

        let z_step_mm = if self.shared_state.cgh_mode == CghMode::CghInplane {-0.1e-3} else {-1.0e-3};
        assert!(z_end_mm < z_start_mm);
        let z_span_mm = z_end_mm - z_start_mm;
        let n_slices = (z_span_mm / z_step_mm).ceil() as usize;
        let z_span_mm = (n_slices - 1) as f64 * z_step_mm;
        let z_end_mm = z_start_mm + z_span_mm;
        let z_561_488_offset_mm = -0.003;

        self.calibration_side_488 = self.spim_data.lock().unwrap().read_calib_coeff("side");
        self.calibration_side_561 = SpimCalibration::new(self.calibration_side_488.volts_per_mm, self.calibration_side_488.volts_at_0mm-self.calibration_side_488.volts_per_mm*z_561_488_offset_mm);
        println!("slope_side: {}", self.calibration_side_488.volts_per_mm);
        println!("intercept_side: {}", self.calibration_side_488.volts_at_0mm);

        let slow_galvo_side_488 = StepRamp::new(StepRampData {
            start_volts: self.calibration_side_488.to_volts(z_start_mm),
            step_volts: self.calibration_side_488.span_to_volts(z_step_mm),
            end_volts: self.calibration_side_488.to_volts(z_end_mm),
            step_s: steptime_slow_galvo,
            offset_s: t0 - 100e-6,
        });
        let slow_galvo_side_561 = StepRamp::new(StepRampData {
            start_volts: self.calibration_side_561.to_volts(z_start_mm),
            step_volts: self.calibration_side_561.span_to_volts(z_step_mm),
            end_volts: self.calibration_side_561.to_volts(z_end_mm),
            step_s: steptime_slow_galvo,
            offset_s: t0 - 100e-6,
        });
        assert_eq!(slow_galvo_side_488.n_steps(), n_slices);
        
        let on_488_s = if self.shared_state.cgh_mode == CghMode::Stack561 {0.0} else {on_laser_s}; 
        // let on_561_s = if self.shared_state.cgh_mode == CghMode::Stack488 || self.shared_state.cgh_mode == CghMode::CghInplane {0.0} else {on_laser_s};
        let on_561_s = if self.shared_state.cgh_mode == CghMode::Stack488 {0.0} else {on_laser_s};
        let laser_period_488 = match self.shared_state.cgh_mode { 
            CghMode::Stack2ch => 2.0 * period_fast,
            _ => period_fast
        };
        let laser_period_561 = match self.shared_state.cgh_mode { 
            CghMode::Stack2ch => 2.0 * period_fast,
            // CghMode::CghInplane => steptime_slow_galvo,
            _ => period_fast
        };
        let offset_488 = t0;
        let offset_561 = match self.shared_state.cgh_mode { 
            CghMode::Stack2ch => period_fast + t0,
            // CghMode::CghInplane => 10.0*period_fast + t0,
            _ => t0
        };
        let obis_488 = Pulse::new(PulseData { period_s: laser_period_488, on_s: on_488_s, offset_s: offset_488 });
        let obis_561 = Pulse::new(PulseData { period_s: laser_period_561, on_s: on_561_s, offset_s: offset_561 });
        let slm_trig = Pulse::new(PulseData { period_s: period_slm, on_s: 1e-3, offset_s: t0 - 100e-6 });
        let spirit_pulsepick = PulseTrain::new(PulseData { period_s: pulse_period, on_s: 1e-3, offset_s: t0 }, pulse_on_times.clone()); //pulse period is 30s (50Hz frame rate) 
        let spim_camera = Pulse::new(PulseData { period_s: period_fast, on_s: period_fast / 5.0, offset_s: t0 - 40e-6 }); // trigger delay is 28.8-36 us, use 40 us to be safe

        self.daq.DO().add_streaming_line(0, move |data, i| obis_488.chunk(data, i));
        self.daq.DO().add_streaming_line(1, move |data, i| obis_561.chunk(data, i));
        self.daq.DO().add_streaming_line(4, move |data, i| spirit_pulsepick.chunk(data, i));
        self.daq.DO().add_streaming_line(2, move |data, i| slm_trig.chunk(data, i));
        self.daq.DO().add_streaming_line(3, move |data, i| spim_camera.chunk(data, i));
        
        match self.shared_state.cgh_mode { 
            CghMode::CghInplane => {
                self.daq.AO().add_streaming_channel(0, move |data, i| fast_galvo_side.chunk(data, i));
                // self.daq.AO().add_streaming_channel(1, move |data, i| slow_galvo_side_488.chunk(data, i));
                self.daq.AO().add_streaming_channel(2, move |data, i| spiral_galvo_x.chunk(data, i));
                self.daq.AO().add_streaming_channel(3, move |data, i| spiral_galvo_y.chunk(data, i));
            },
            CghMode::Stack488 => {
                self.daq.AO().add_streaming_channel(0, move |data, i| fast_galvo_side.chunk(data, i));
                self.daq.AO().add_streaming_channel(1, move |data, i| slow_galvo_side_488.chunk(data, i));
            },
            CghMode::Stack561 => {
                self.daq.AO().add_streaming_channel(0, move |data, i| fast_galvo_side.chunk(data, i));
                self.daq.AO().add_streaming_channel(1, move |data, i| slow_galvo_side_561.chunk(data, i));
            },
            CghMode::Stack2ch => {
                self.daq.AO().add_streaming_channel(0, move |data, i| fast_galvo_side.chunk(data, i));
                self.daq.AO().add_streaming_channel(1, move |data, i| slow_galvo_side_488.chunk(data, i));
            },
            CghMode::SpimCalib => {
                self.daq.AO().add_streaming_channel(0, move |data, i| fast_galvo_side.chunk(data, i));
            },
            _ => {
                println!("Ondemand");
            }
        };
        let dout = daq.DO();
        let aout = daq.AO();
        dout.set(0);
        aout.set(0, 0.0f32);
        aout.set(1, 0.0f32);
        aout.set(2, 0.0f32);
        aout.set(3, 0.0f32);
        // let capacity = 200;
        // let warn_threshold = 10;

        let shared_state = self.shared_state.clone();
        let camera = self.camera.clone();
        let sample_z_stage: Arc<PmdDevice> = self.sample_z_stage.clone();
        let stream = self.stream.clone();
        let fl_filename: PathBuf = self.fl_path.clone().join("fl.bin");
        let spim_data = self.spim_data.clone();
        let size = self.size.clone();
        let slm = self.slm.clone();
        let slm_path = self.slm_path.clone();        
        let n_buffers = 3;
        let mut fl_writer = AsyncWriter::new(fl_filename, n_buffers).unwrap();

        self.thread = Some(spawn(move || {
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                set_backend(Backend::CUDA); // Or Backend::OPENCL
                set_device(0);
                unsafe {
                    cudaSetDevice(gpu_index);
                }
                if shared_state.cgh_mode == CghMode::Stack488 || shared_state.cgh_mode == CghMode::Stack561 || shared_state.cgh_mode == CghMode::CghInplane {
                    sample_z_stage.move_abs_sync(z_start_mm as f32);
                }
                let rx = camera.lock().unwrap().start(thread_panic.clone(), shared_state.frame_rate.clone());
                if shared_state.cgh_mode != CghMode::Ondemand {
                    println!("Starting AO");
                    aout.start_as_follower(true);
                }
                dout.start();

                let phasemask_filename = slm_path.join("cgh.bin");
                let tgt_image_filename = slm_path.join("tgt_img.bin");
                let zmq_server_on = shared_state.cgh_mode == CghMode::CghInplane || shared_state.cgh_mode == CghMode::SpimCalib;
                for image in rx {
                    let z_stack = shared_state.cgh_mode == CghMode::Stack488 || shared_state.cgh_mode == CghMode::Stack561 || shared_state.cgh_mode == CghMode::Stack2ch || shared_state.cgh_mode == CghMode::CghInplane;
                    let dcam_frame_index = image.frame_index;
                    let sweep_index = dcam_frame_index as usize / n_slices;
                    let slice_index = match shared_state.cgh_mode {
                        CghMode::CghInplane => (dcam_frame_index as usize / (n_pulses as f64*pulse_period/period_fast) as usize) as usize % n_slices,
                        CghMode::Stack2ch => (dcam_frame_index as usize / 2 as usize) % n_slices,
                        _ => dcam_frame_index as usize % n_slices
                    };
                    if shared_state.sample_z_home.load(Relaxed) { 
                        sample_z_stage.home();
                        shared_state.sample_z_home.store(false, Relaxed);
                    }
                    let z_target_mm = if z_stack { (z_start_mm + slice_index as f64 * z_step_mm) as f32 } else { shared_state.sample_z_manual_target_mm.load() }; 
                    
                    match shared_state.cgh_mode {
                        CghMode::CghInplane => {
                            if image.frame_index % 20 == 0 {
                                sample_z_stage.move_abs_fast(z_target_mm);
                                shared_state.sample_z_position_mm.store(sample_z_stage.actual_position());
                            }
                        },
                        CghMode::Stack2ch => {
                            if image.frame_index % 2 == 0 {
                                sample_z_stage.move_abs_fast(z_target_mm);
                                shared_state.sample_z_position_mm.store(sample_z_stage.actual_position());
                            }
                        },
                        _ => {
                            sample_z_stage.move_abs_fast(z_target_mm);
                            shared_state.sample_z_position_mm.store(sample_z_stage.actual_position()); 
                        }
                    };

                    let image = Arc::new(image);
                    if image.frame_index % 10 == 0 {
                        // println!("image {}", image.frame_index);
                        let roi = &image.data;
                        unsafe {
                            let d_roi_mut = &mut *(&shared_state.d_roi as *const DeviceBuffer<u16> as *mut DeviceBuffer<u16>);
                            d_roi_mut.async_copy_from(roi, &stream).unwrap()
                        };
                    }
                    // generate SLM control pulse and calculate hologram 
                    match shared_state.cgh_mode {
                        CghMode:: CghInplane => {
                            if shared_state.generate_new_holo.load(Relaxed) {
                                // slm.lock().unwrap().read_target_img_file(&tgt_image_filename);
                                // slm.lock().unwrap().calc_gs2d(10);
                                // let cghx:i32 = ( (shared_state.shift_3d.load().0 as f32 - (size.0 as f32)/2.0) * (slm.lock().unwrap().slm_size.0 as f32/size.0 as f32) ).floor() as i32;  
                                // let cghy:i32 = ( (shared_state.shift_3d.load().1 as f32 - (size.1 as f32)/2.0) * (slm.lock().unwrap().slm_size.1 as f32/size.1 as f32) ).floor() as i32; 
                                // let cghx:i32 = ( (shared_state.shift_3d.load().0 as f32 - shared_state.cgh_zero_ord.load().0 as f32) * (slm.lock().unwrap().slm_size.0 as f32/size.0 as f32) ).floor() as i32;  
                                // let cghy:i32 = ( (shared_state.shift_3d.load().1 as f32 - shared_state.cgh_zero_ord.load().1 as f32) * (slm.lock().unwrap().slm_size.1 as f32/size.1 as f32) ).floor() as i32; 
                                let cghx:i32 = ( (shared_state.shift_3d.load().0 as f32 - shared_state.cgh_zero_ord.load().0 as f32)/3.0).floor() as i32;  
                                let cghy:i32 = ( (shared_state.shift_3d.load().1 as f32 - shared_state.cgh_zero_ord.load().1 as f32)/3.0).floor() as i32; 
                                let cghz:i32 = shared_state.shift_3d.load().2;
                                println!("cghX: {}", cghy);
                                println!("cghY: {}", cghx);
                                println!("Zero ord X: {}", shared_state.cgh_zero_ord.load().0);
                                println!("Zero ord Y: {}", shared_state.cgh_zero_ord.load().1);
                                slm.lock().unwrap().calc_superpos3d((cghx, cghy, cghz));
                                let _ = slm.lock().unwrap().write_phase_mask_file(&phasemask_filename);
                                shared_state.generate_new_holo.store(false, Relaxed);
                            }
                        },
                        CghMode:: SpimCalib => {
                            if shared_state.generate_new_holo.load(Relaxed) {
                                let cghx:i32 = ( (shared_state.shift_3d.load().0 as f32 - shared_state.cgh_zero_ord.load().0 as f32)/3.0).floor() as i32;  
                                let cghy:i32 = ( (shared_state.shift_3d.load().1 as f32 - shared_state.cgh_zero_ord.load().1 as f32)/3.0).floor() as i32; 
                                let cghz:i32 = shared_state.shift_3d.load().2;
                                println!("cghX: {}", cghy);
                                println!("cghY: {}", cghx);
                                println!("Zero ord X: {}", shared_state.cgh_zero_ord.load().0);
                                println!("Zero ord Y: {}", shared_state.cgh_zero_ord.load().1);
                                slm.lock().unwrap().calc_superpos3d((cghx, cghy, cghz));
                                let _ = slm.lock().unwrap().write_phase_mask_file(&phasemask_filename);
                                shared_state.generate_new_holo.store(false, Relaxed);
                            } else {
                                slm.lock().unwrap().send_pong();
                            }
                        },
                        _ => {
                            // println!("CGH Calc TBD");
                        }
                    }

                    if shared_state.cgh_mode == CghMode::Ondemand {
                        aout.set(0, shared_state.ao0.load());
                        aout.set(1, shared_state.ao1.load());
                        aout.set(2, shared_state.ao2.load());
                        aout.set(3, shared_state.ao3.load());
                    }
                    if shared_state.cgh_mode == CghMode::SpimCalib {
                        aout.set(1, shared_state.ao1.load());
                    }
                    if shared_state.cgh_mode == CghMode::CghInplane {
                        aout.set(1, shared_state.ao1.load());
                    }
                    let save_divisor = shared_state.save_divisor.load(Relaxed);
                    // debug!("Writing fl image");
                    if z_stack && (sweep_index % save_divisor as usize == 0) { 
                        fl_writer.write_all_u16(image.data).unwrap();
                    }
                    if shared_state.save_img.load(Relaxed) {
                        fl_writer.write_all_u16(image.data).unwrap();
                        shared_state.save_img.store(false, Relaxed);    
                    }
                    if shared_state.start_calib.load(Relaxed) {
                        spim_data.lock().unwrap().create_calib_file();
                        shared_state.start_calib.store(false, Relaxed);
                    }
                    if shared_state.save_calib.load(Relaxed) {
                        spim_data.lock().unwrap().update_vec(shared_state.sample_z_position_mm.load(), shared_state.ao1.load(), shared_state.ao1.load());
                        shared_state.n_datapts.store(shared_state.n_datapts.load()+1);
                        shared_state.save_calib.store(false, Relaxed);
                    }
                    if shared_state.stop_calib.load(Relaxed) {
                        spim_data.lock().unwrap().write_calib_data();
                        shared_state.stop_calib.store(false, Relaxed);
                    }
                    if shared_state.save_expt.load(Relaxed) {
                        spim_data.lock().unwrap().create_expt_params_file(z_start_mm, z_end_mm, z_step_mm, period_fast, &pulse_on_times);
                        shared_state.save_expt.store(false, Relaxed);
                    }
                    if stop.load(Relaxed) || thread_panic.load(Relaxed) {
                        break;
                    }
                }
            }));
            if result.is_err() {
                info!("Spim: camera thread panic");
                thread_panic.store(true, Relaxed);
            }
            fl_writer.flush().unwrap();
            aout.stop();
            dout.stop();
            dout.set(0);
            aout.set(0, 0.0f32);
            aout.set(1, 0.0f32);
            aout.set(2, 0.0f32);
            aout.set(3, 0.0f32);
        }));
    }
}

impl Drop for Cgh {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            debug!("Cgh join");
            thread.join().unwrap();
            debug!("Cgh joined");
        }
    }
}

use crate::{CghMode, SharedState, CghPositions};
use std::rc::Rc;
use crossbeam::atomic::AtomicCell;
use cust::prelude::*;
use cust::{memory::DeviceBuffer, stream::Stream};
use glium::backend::Facade;
use glium::{glutin, Display};
use imgui::*;
use imgui_glium_renderer::Texture;
use imgui_utils::{AnyTexture, CuTextureF32, CuTextureI16, CuTextureU16, CuTextureU16U16, CuTextureU8, CustomRenderer};
use npp::StreamContext;
use std::sync::{
    atomic::Ordering::{Relaxed, Release},
    Arc,
};

use super::shared_state;

pub struct CghUI {
    cgh_texture: Rc<CuTextureU16>,
    id_cgh_texture: TextureId,
    cgh_positions: CghPositions,
    size: (usize, usize),
    cgh_stream: Arc<Stream>,
}

impl CghUI {
    pub fn new(size: (usize, usize), cgh_texture: Rc<CuTextureU16>, id_cgh_texture: TextureId, cgh_stream: Arc<Stream>) -> Self {
        // let v_min = Rc::new(AtomicCell::new(0.0f32));
        // let v_max = Rc::new(AtomicCell::new(10000.0f32));
        let cgh_positions: CghPositions = CghPositions::new();
        Self{   cgh_texture,
                id_cgh_texture,
                cgh_positions, 
                size,
                cgh_stream,
            }
    }
}

impl CghUI {
    pub fn show(&mut self, ui: &Ui, shared_state: &SharedState) {
        let v_min = 0.0;
        let mut v_max = shared_state.v_max.load();
        let mut sample_z_target = shared_state.sample_z_manual_target_mm.load();
    
        if Slider::new(ui, "sample_z", -2.0, 2.0).build(&mut sample_z_target) {
            shared_state.sample_z_manual_target_mm.store(sample_z_target);
        }
        ui.same_line();
        if ui.button("Home") {
            shared_state.sample_z_home.store(true, Relaxed);
        }
        ui.same_line();
        if ui.button("Off") {
            shared_state.sample_z_off.store(true, Relaxed);
        }
        ui.same_line();
        if ui.button("On") {
            shared_state.sample_z_on.store(true, Relaxed);
        }
        ui.same_line();
        let event_status = shared_state.sample_z_event_status.load(Relaxed);
        ui.text(format!("{event_status:#010b}"));
        ui.same_line();
        let position_mm = shared_state.sample_z_position_mm.load();
        ui.text(format!("{position_mm:.4} mm"));

        if Slider::new(ui, "v_max", 1f32, 65535f32).build(&mut v_max) {
            shared_state.v_max.store(v_max);
        }
        let mut save_divisor = shared_state.save_divisor.load(Relaxed);
        /* if Slider::new("save_divisor", 1, 10).build(ui, &mut save_divisor) {
            shared_state.save_divisor.store(save_divisor, Relaxed);
        }*/
        let mut shift_3d = shared_state.shift_3d.load();
        let mut shift_x = shift_3d.0;
        let mut shift_y = shift_3d.1;
        let mut shift_z = shift_3d.2;
        if Slider::new(ui, "cgh_shift_z", -100, 100).build(&mut shift_z) {
            shared_state.shift_3d.store((shift_x,shift_y,shift_z));
        }
        ui.same_line();
        if ui.button("save_zero_ord") {
            shared_state.save_zero_ord.store(true, Release);
        }
        ui.same_line();if ui.button("enable_click_cgh") {
            shared_state.enable_click_cgh.store(true, Release);
        }
        ui.same_line();
        if ui.button("disable_click_cgh") {
            shared_state.enable_click_cgh.store(false, Release);
        }

        if ui.button("save curr_image") {
            shared_state.save_img.store(true, Release);
        }
        if shared_state.cgh_mode == CghMode::Ondemand {
            let mut v_max_ao0 = shared_state.ao0.load();
            if Slider::new(ui, "AO0:side_fast", -10f32, 10f32).build(&mut v_max_ao0) {
                shared_state.ao0.store(v_max_ao0);
            }
            let mut v_max_ao1 = shared_state.ao1.load();
            if Slider::new(ui, "AO1:side_slow", -10f32, 10f32).build(&mut v_max_ao1) {
                shared_state.ao1.store(v_max_ao1);
            }
            let mut v_max_ao2 = shared_state.ao2.load();
            if Slider::new(ui, "AO2:spiral_x", -1f32, 1f32).build(&mut v_max_ao2) {
                shared_state.ao2.store(v_max_ao2);
            }
            let mut v_max_ao3 = shared_state.ao3.load();
            if Slider::new(ui, "AO3:spiral_y", -1f32, 1f32).build(&mut v_max_ao3) {
                shared_state.ao3.store(v_max_ao3);
            }
        } else if shared_state.cgh_mode == CghMode::SpimCalib {
            let mut v_max_ao1 = shared_state.ao1.load();
            if Slider::new(ui, "AO1:side", -5f32, 5f32).build(&mut v_max_ao1) {
                shared_state.ao1.store(v_max_ao1);
            }
            if ui.button("start_calib") {
                shared_state.start_calib.store(true, Release);
            }
            ui.same_line();
            if ui.button("save_datapoint") {
                shared_state.save_calib.store(true, Release);
            }
            ui.same_line();
            if ui.button("stop_calib") {
                shared_state.stop_calib.store(true, Release);
            }
            ui.same_line();
            ui.text(format!("No of Datapoints: {}", shared_state.n_datapts.load()));
        } else if shared_state.cgh_mode == CghMode::CghInplane {
            let mut v_max_ao1 = shared_state.ao1.load();
            if Slider::new(ui, "AO1:side", -5f32, 5f32).build(&mut v_max_ao1) {
                shared_state.ao1.store(v_max_ao1);
            }
        } else {
            if ui.button("save_expt_params") {
                shared_state.save_expt.store(true, Release);
            }
        }

        let scale = 1.0;
        let p1 = ui.cursor_pos();
        /*let AnyTexture::CuU16(cgh_texture) = renderer.lookup_texture(id_cgh_texture).unwrap() else {
             unreachable!();
        }; */
        self.cgh_texture.update_from_cuda_async(&shared_state.d_roi, &self.cgh_stream);
        imgui::Image::new(self.id_cgh_texture, [self.size.0 as f32 * scale, self.size.1 as f32 * scale]).build(ui);

        if shared_state.save_zero_ord.load(Relaxed) {
            let mut mouse_pos = ui.io().mouse_pos;
            if  (mouse_pos[1] + ui.scroll_y()) > p1[1] {
                if ui.io().want_capture_mouse { 
                    if ui.is_mouse_clicked(MouseButton::Left) {
                        mouse_pos = ui.io().mouse_pos;
                        let mut shift_x = (scale*mouse_pos[0]+ui.scroll_x() - p1[0]).floor() as i32;
                        let mut shift_y = (scale*mouse_pos[1]+ui.scroll_y() - p1[1]).floor() as i32;
                        shared_state.cgh_zero_ord.store((shift_x,shift_y));
                        shared_state.save_zero_ord.store(false, Relaxed); 
                    }
                } 
            }
        }
        if shared_state.cgh_zero_ord.load().0 > 0 {
            ui.get_foreground_draw_list().add_circle([shared_state.cgh_zero_ord.load().0 as f32+p1[0] as f32, shared_state.cgh_zero_ord.load().1 as f32+p1[1] as f32], 5.0, [0.0, 1.0, 1.0]).thickness(2.0).build();
        }

        if shared_state.enable_click_cgh.load(Relaxed) {
            let mut mouse_pos = ui.io().mouse_pos;
            if  (mouse_pos[1] + ui.scroll_y()) > p1[1] {
                if ui.io().want_capture_mouse { 
                    if ui.is_mouse_clicked(MouseButton::Left) {
                        mouse_pos = ui.io().mouse_pos;
                        self.cgh_positions.update_vec(scale*mouse_pos[0]+ui.scroll_x(), scale*mouse_pos[1]+ui.scroll_y());
                        let mut shift_x = (scale*mouse_pos[0]+ui.scroll_x() - p1[0]).floor() as i32;
                        let mut shift_y = (scale*mouse_pos[1]+ui.scroll_y() - p1[1]).floor() as i32;
                        shared_state.shift_3d.store((shift_x,shift_y,shift_z));
                        shared_state.generate_new_holo.store(true, Release);
                    }
                } 
            } 
        }
       
        for (&pos_x, &pos_y) in self.cgh_positions.clicks_x.iter().zip(self.cgh_positions.clicks_y.iter()) {
            ui.get_foreground_draw_list().add_circle([pos_x-ui.scroll_x(), pos_y-ui.scroll_y()], 5.0, [1.0, 0.0, 0.0]).thickness(2.0).build();
        }

        ui.text(format!("dcam: {}°", shared_state.camera_temperature.load()));
        ui.text(format!("dcam: {} fps", shared_state.frame_rate.load()));
    }
}







use crate::{CghMode, SharedState, CghPositions};
use std::rc::Rc;
use std::time::Instant;
use crossbeam::atomic::AtomicCell;
use cust::prelude::*;
use cust::{memory::DeviceBuffer, stream::Stream};
use glium::backend::Facade;
use glium::{glutin, Display, Surface};
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::platform::run_return::EventLoopExtRunReturn;
use glium::glutin::window::WindowBuilder;
use glutin::{GlRequest, GlProfile};
use imgui::{Condition, Window};
use imgui::{FontConfig, FontSource};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use imgui_utils::{AnyTexture, CuTextureF32, CuTextureI16, CuTextureU16, CuTextureU16U16, CustomRenderer};
use imgui::*;
use imgui_glium_renderer::Texture;
use npp::StreamContext;
use std::sync::{
    atomic::Ordering::{Relaxed, Release},
    Arc,
};
use std::cell::{RefCell, RefMut};

use super::{cgh, shared_state};

/* 
pub struct CghTexture {
    pub cu_texture: CuTextureU16,
}

impl CghTexture {
    pub fn new(gl_context: &Rc<glium::backend::Context>, size: (usize, usize), cgh_v_max: Rc<AtomicCell<f32>>) -> Self {
        let mut cu_texture = CuTextureU16::new(gl_context, size, Rc::new(AtomicCell::new(0.0)), cgh_v_max);
        Self {   cu_texture,
        }
    }
    pub fn get_texture_id(&mut self, mut renderer: CustomRenderer) {
        let textures = renderer.textures();
        let id_cgh_texture = textures.insert(AnyTexture::CuU16(self.cu_texture));
    }
}
*/
pub struct CghUI {
    size: (usize, usize),
    cgh_stream: Arc<Stream>,
    cgh_positions: CghPositions,
}

impl CghUI {
    pub fn new(size: (usize, usize), cgh_stream: Arc<Stream>) -> Self {
        let cgh_positions: CghPositions = CghPositions::new();
        Self{   size,
                cgh_stream,
                cgh_positions, 
            }
    }

    pub fn run_event_loop(&mut self, shared_state: &SharedState, mut last_frame: Instant) {
        let title = "cgh";
        let mut event_loop = EventLoop::new();
        let context = glutin::ContextBuilder::new().with_vsync(true);
        /* let context = glutin::ContextBuilder::new().with_vsync(true)
                .with_gl(GlRequest::Latest).with_gl_profile(GlProfile::Compatibility)
                .with_depth_buffer(24)
                .with_srgb(false); 
        */
        let builder = WindowBuilder::new().with_title(title.to_owned());
        let display = Display::new(builder, context, &event_loop).expect("Failed to initialize display");
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);
        let mut platform = WinitPlatform::init(&mut imgui);
        {
            let gl_window = display.gl_window();
            let window = gl_window.window();
            window.set_maximized(true);
            platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);
        }
        let hidpi_factor = platform.hidpi_factor() as f32;
        let font_size = 13.0 * hidpi_factor;

        imgui.fonts().add_font(&[FontSource::DefaultFontData { config: Some(FontConfig { size_pixels: font_size, ..FontConfig::default() }) }]);
        imgui.io_mut().font_global_scale = 1.0 / hidpi_factor;
        let mut renderer = CustomRenderer::init(&mut imgui, &display).expect("Failed to initialize renderer");
        let gl_context = display.get_context();
        let cgh_v_max = Rc::new(AtomicCell::new(10000.0));
        
        let mut cgh_texture = CuTextureU16::new(gl_context, self.size, Rc::new(AtomicCell::new(0.0)), cgh_v_max.clone());
        // let cgh_texture = CghTexture::new(gl_context, renderer, self.size, cgh_v_max);
        // let mut cgh_texture = RefCell::new(CuTextureU16::new(gl_context, self.size, Rc::new(AtomicCell::new(0.0)), cgh_v_max.clone()));
        // let mut cgh_texture = Rc::new(CuTextureU16::new(gl_context, self.size, Rc::new(AtomicCell::new(0.0)), cgh_v_max.clone()));
        // let mut cgh_texture: Rc<RefCell<CuTextureU16>> = Rc::new(RefCell::new(CuTextureU16::new(gl_context, self.size, Rc::new(AtomicCell::new(0.0)), cgh_v_max.clone())));
        let textures = renderer.textures();
        let id_cgh_texture: TextureId = textures.insert(AnyTexture::CuU16(cgh_texture));
        // let mut cgh_texture_c = cgh_texture.clone();

        event_loop.run_return(move |event, _, control_flow| match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                imgui.io_mut().update_delta_time(now - last_frame);
                last_frame = now;
            }
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();
                platform.prepare_frame(imgui.io_mut(), gl_window.window()).expect("Failed to prepare frame");
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let ui = imgui.frame();
                let size = display.gl_window().window().inner_size();
                let size = [size.width as f32 / hidpi_factor, size.height as f32 / hidpi_factor];
                ui.window("Main").position([0.0, 0.0], imgui::Condition::Always).size(size, Condition::Always).no_decoration().movable(false).build(|| {
                    // self.show(ui, &shared_state, Rc::get_mut(&mut cgh_texture_c).unwrap());
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
                    let AnyTexture::CuU16(cgh_texture) = renderer.lookup_texture(id_cgh_texture).unwrap() else {
                        unreachable!();
                    };
                    cgh_texture.update_from_cuda_async(&shared_state.d_roi, &self.cgh_stream);
                    imgui::Image::new(id_cgh_texture, [self.size.0 as f32 * scale, self.size.1 as f32 * scale]).build(ui);
                    
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
                });
                /* if stop.load(Relaxed) {
                    *control_flow = ControlFlow::Exit;
                } */
                let gl_window = display.gl_window();
                let mut target = display.draw();
                target.clear_color_srgb(1.0, 1.0, 1.0, 1.0);
                platform.prepare_render(ui, gl_window.window());
                let draw_data = imgui.render();
                renderer.render(&mut target, draw_data).expect("Rendering failed");
                target.finish().expect("Failed to swap buffers");
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            event => {
                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
            }
        });

    }

    /*  
    fn show(&mut self, ui: &Ui, shared_state: &SharedState,  cgh_texture: &mut CuTextureU16) {
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
        /* let AnyTexture::CuU16(cgh_texture) = renderer.lookup_texture(id_cgh_texture).unwrap() else {
            unreachable!();
        };
        cgh_texture.update_from_cuda_async(&shared_state.d_roi, &self.cgh_stream);
        imgui::Image::new(id_cgh_texture, [self.size.0 as f32 * scale, self.size.1 as f32 * scale]).build(ui);
        */
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
    */
}







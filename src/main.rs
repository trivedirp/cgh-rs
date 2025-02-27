#![allow(warnings)] 
use clap::Parser;
use log::{debug, info};
use cust::stream::{Stream, StreamFlags};
use data::{data_raw_timestamp, new_data_raw_path, new_timestamp};
use glium::backend::Facade;
use glium::glutin;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::platform::run_return::EventLoopExtRunReturn;
use glium::glutin::window::WindowBuilder;
use glium::{Display, Surface};
use imgui::{Condition, Window};
use imgui::{FontConfig, FontSource};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use imgui_utils::{AnyTexture, CuTextureF32, CuTextureI16, CuTextureU16, CuTextureU16U16, CustomRenderer};
use npp::StreamContext;
use cgh::{CghMode, Cgh, CghUI};
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc,
};
use crossbeam::atomic::AtomicCell;
use std::rc::Rc;
use std::time::Instant;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]

struct Args {
    #[clap(arg_enum, default_value_t = CghMode::Ondemand)]
    mode: CghMode,
}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Info).init();
    // env_logger::builder().filter_level(log::LevelFilter::Debug).init();
    let args = Args::parse();
    let gpu_index = 0;
    let _ctx = cust::quick_init().unwrap();
    let timestamp = new_timestamp();
    let fl_path: Arc<std::path::PathBuf> = Arc::new(data_raw_timestamp("/data", true, &timestamp));
    let cgh_stream = Arc::new(Stream::new(StreamFlags::NON_BLOCKING, None).unwrap());
    let _cgh_stream_context = StreamContext::new(&cgh_stream);
    let mut cgh = Cgh::new(args.mode, fl_path.clone());
    let cgh_state = cgh.shared_state.clone();
    /* 
    let ui_stream = Arc::new(Stream::new(StreamFlags::NON_BLOCKING, None).unwrap());
    let ui_stream_context = StreamContext::new(&ui_stream);
    let mut system = imgui_utils::init("cgh");
    let hidpi_factor = system.platform.hidpi_factor() as f32;
    let gl_context = system.display.get_context();
    let textures = system.renderer.textures();
    let mut cgh_ui = CghUI::new(cgh.size, gl_context, textures, ui_stream, ui_stream_context);
    */
    let title = "cgh";
    let mut event_loop = EventLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
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
    let cgh_texture = Rc::new(CuTextureU16::new(gl_context, cgh.size, Rc::new(AtomicCell::new(0.0)), cgh_v_max.clone()));
    let cgh_texture_c = cgh_texture.clone();
    let textures = renderer.textures();
    let id_cgh_texture = textures.insert(AnyTexture::CuU16(Rc::into_inner(cgh_texture).unwrap()));
    let mut cgh_ui = CghUI::new(cgh.size, cgh_texture_c, id_cgh_texture, cgh_stream);
    
    let thread_panic = Arc::new(AtomicBool::new(false));
    let stop_cgh = Arc::new(AtomicBool::new(false));
    cgh.start(gpu_index, stop_cgh.clone(), thread_panic.clone());
    /*  
    system.main_loop(move |_run, ui, display| {
        let size = display.gl_window().window().inner_size();
        let size = [size.width as f32 / hidpi_factor, size.height as f32 / hidpi_factor];
        Window::new(&ui, "Main").position([0.0, 0.0], imgui::Condition::Always).size(size, Condition::Always).no_decoration().movable(false).build( || {
            cgh_ui.show(ui, &cgh_state);
        });
    }); */
    let mut last_frame = Instant::now();
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
                cgh_ui.show(ui, &cgh_state);
            });
        }
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            let gl_window = display.gl_window();
            platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
        }
    });
    stop_cgh.store(true, Relaxed);
}

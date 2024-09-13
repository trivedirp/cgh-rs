#![allow(warnings)] 
use clap::Parser;
use log::{debug, info};
use cust::stream::{Stream, StreamFlags};
use data::{data_raw_timestamp, new_data_raw_path, new_timestamp};
use glium::backend::Facade;
use imgui::{Condition, Window};
use npp::StreamContext;
use cgh::{CghMode, Cgh, CghUI};
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc,
};

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
    let mut cgh = Cgh::new(args.mode, cgh_stream, fl_path.clone());

    let cgh_state = cgh.shared_state.clone();

    let ui_stream = Arc::new(Stream::new(StreamFlags::NON_BLOCKING, None).unwrap());
    let ui_stream_context = StreamContext::new(&ui_stream);
    let mut system = imgui_utils::init("cgh");
    let hidpi_factor = system.platform.hidpi_factor() as f32;
    let gl_context = system.display.get_context();
    let textures = system.renderer.textures();
    let mut cgh_ui = CghUI::new(cgh.size, gl_context, textures, ui_stream, ui_stream_context);
    let thread_panic = Arc::new(AtomicBool::new(false));
    let stop_cgh = Arc::new(AtomicBool::new(false));
    cgh.start(gpu_index, stop_cgh.clone(), thread_panic.clone());
    system.main_loop(move |_run, ui, display| {
        let size = display.gl_window().window().inner_size();
        let size = [size.width as f32 / hidpi_factor, size.height as f32 / hidpi_factor];
        Window::new("Main").position([0.0, 0.0], imgui::Condition::Always).size(size, Condition::Always).no_decoration().movable(false).build(ui, || {
            cgh_ui.show(ui, &cgh_state);
        });
    });
    stop_cgh.store(true, Relaxed);
}

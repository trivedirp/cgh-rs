#![allow(warnings)] 

use clap::Parser;
use log::{debug, info};
use cust::stream::{Stream, StreamFlags};
use data::{data_raw_timestamp, new_data_raw_path, new_timestamp};
use npp::StreamContext;
use cgh::{CghMode, Cgh, CghUI};
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc,
};
use crossbeam::atomic::AtomicCell;
use std::rc::Rc;

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
    let cgh_stream = Arc::new(Stream::new(StreamFlags::NON_BLOCKING, None).unwrap());
    let _cgh_stream_context = StreamContext::new(&cgh_stream);
    let mut cgh = Cgh::new(args.mode, cgh_stream.clone());
    let cgh_state = cgh.shared_state.clone();
    let thread_panic = Arc::new(AtomicBool::new(false));
    let stop_cgh = Arc::new(AtomicBool::new(false));
    let mut cgh_ui = CghUI::new(cgh.size, cgh_stream);
    cgh.start(gpu_index, stop_cgh.clone(), thread_panic.clone());

    cgh_ui.run_event_loop(&cgh_state);
    
    stop_cgh.store(true, Relaxed);
}


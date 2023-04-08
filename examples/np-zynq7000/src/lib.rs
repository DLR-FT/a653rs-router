#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_rs::prelude::ApexTimeP1Ext;
use apex_rs_xng::apex::XngHypervisor;
use log::LevelFilter;
use small_trace_gpio::GpioTracer;
use xng_rs_log::XalLogger;

include!(concat!(env!("OUT_DIR"), "/np.rs"));

static LOGGER: XalLogger = XalLogger;
static TRACER: GpioTracer = GpioTracer::new();

#[no_mangle]
pub extern "C" fn main() {
    // The logger should be disabled during measurements
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(log::LevelFilter::Info);
    //TRACER.init();
    //unsafe { small_trace::set_tracer(&TRACER) }
    NetworkPartition.run();
}

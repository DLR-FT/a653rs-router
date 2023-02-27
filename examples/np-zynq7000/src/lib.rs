#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_rs::prelude::ApexTimeP1Ext;
use apex_rs_xng::apex::XngHypervisor;
use log::LevelFilter;
use network_partition_xng::GpioTracer;
use xng_rs_log::log::XalLogger;

include!(concat!(env!("OUT_DIR"), "/np.rs"));

static LOGGER: XalLogger = XalLogger;
static TRACER: GpioTracer = GpioTracer::new();

#[no_mangle]
pub extern "C" fn main() {
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    TRACER.init();
    unsafe { small_trace::set_tracer(&TRACER) }
    log::set_max_level(log::LevelFilter::Trace);
    NetworkPartition.run();
}

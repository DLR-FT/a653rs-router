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
    // This is trying to use memory that is
    // - not delegated to this partition in XNG
    // - belongs to MMIO that is used for other peripherals on CoraZ7
    // - should use AXI GPIO instead, so IO-15.. can be used.
    //unsafe { one_byte_trace::set_tracer(&TRACER) }
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(log::LevelFilter::Trace);
    NetworkPartition.run();
}

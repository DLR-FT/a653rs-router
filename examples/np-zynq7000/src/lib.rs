#![no_std]
#![allow(incomplete_features)]

use xng_rs_log::XalLogger;

include!(concat!(env!("OUT_DIR"), "/np.rs"));

static LOGGER: XalLogger = XalLogger;
// static TRACER: GpioTracer = small_trace_gpio::GpioTracer::new();

#[no_mangle]
pub extern "C" fn main() {
    // The logger should be disabled during measurements
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(log::LevelFilter::Info);
    // TRACER.init();
    // unsafe { small_trace::set_tracer(&TRACER) }
    NetworkPartition.run();
}

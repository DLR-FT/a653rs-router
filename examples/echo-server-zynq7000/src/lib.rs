#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_echo::server_queuing::*;
use apex_rs::prelude::*;
use apex_rs_xng::apex::XngHypervisor;
use coraz7::GpioTracer;
use log::info;
use once_cell::unsync::OnceCell;
use xng_rs_log::XalLogger;

const ECHO_SIZE: MessageSize = 100;
const ECHO_RANGE: MessageRange = 10;

static mut SENDER: OnceCell<QueuingPortSender<ECHO_SIZE, ECHO_RANGE, XngHypervisor>> =
    OnceCell::new();

static mut RECEIVER: OnceCell<QueuingPortReceiver<ECHO_SIZE, ECHO_RANGE, XngHypervisor>> =
    OnceCell::new();

static LOGGER: XalLogger = XalLogger;

static TRACER: GpioTracer = GpioTracer::new();

#[no_mangle]
pub extern "C" fn main() {
    // The logger should be disabled during measurements
    //unsafe { log::set_logger_racy(&XalLogger) };
    //log::set_max_level(log::LevelFilter::Info);
    TRACER.init();
    unsafe { small_trace::set_tracer(&TRACER) }
    info!("Echo server main");
    let partition = EchoServerPartition::new(
        unsafe { &SENDER },
        unsafe { &RECEIVER },
        entry_point_aperiodic,
    );
    partition.run()
}

extern "C" fn entry_point_aperiodic() {
    EchoServerProcess::run(unsafe { SENDER.get_mut().unwrap() }, unsafe {
        RECEIVER.get_mut().unwrap()
    });
}

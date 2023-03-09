#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_echo::queuing::*;
use apex_rs::prelude::*;
use apex_rs_xng::apex::XngHypervisor;
use coraz7::{GpioTracer, XalLogger};
use log::info;
use once_cell::unsync::OnceCell;

const ECHO_SIZE: MessageSize = 100;
const FIFO_DEPTH: MessageRange = 10;

static mut SENDER: OnceCell<QueuingPortSender<ECHO_SIZE, FIFO_DEPTH, XngHypervisor>> =
    OnceCell::new();
static mut RECEIVER: OnceCell<QueuingPortReceiver<ECHO_SIZE, FIFO_DEPTH, XngHypervisor>> =
    OnceCell::new();
static LOGGER: XalLogger = XalLogger;
static TRACER: GpioTracer = GpioTracer::new();

#[no_mangle]
pub extern "C" fn main() {
    TRACER.init();
    unsafe { small_trace::set_tracer(&TRACER) }
    // The logger should be disabled during measurements
    //unsafe { log::set_logger_racy(&XalLogger) };
    //log::set_max_level(log::LevelFilter::Info);
    info!("Echo client main");
    let partition = QueuingPeriodicEchoPartition::new(
        unsafe { &SENDER },
        unsafe { &RECEIVER },
        entry_point_periodic,
        entry_point_aperiodic,
    );
    partition.run()
}

extern "C" fn entry_point_periodic() {
    QueuingEchoSender::run(unsafe { SENDER.get_mut().unwrap() });
}

extern "C" fn entry_point_aperiodic() {
    QueuingEchoReceiver::run(unsafe { RECEIVER.get_mut().unwrap() });
}

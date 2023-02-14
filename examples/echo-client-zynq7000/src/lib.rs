#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_echo::client::*;
use apex_rs::prelude::*;
use apex_rs_xng::apex::XngHypervisor;
use log::info;
use once_cell::unsync::OnceCell;
use xng_rs_log::log::XalLogger;

const ECHO_SIZE: MessageSize = 100;

static mut SENDER: OnceCell<SamplingPortSource<ECHO_SIZE, XngHypervisor>> = OnceCell::new();
static mut RECEIVER: OnceCell<SamplingPortDestination<ECHO_SIZE, XngHypervisor>> = OnceCell::new();
static LOGGER: XalLogger = XalLogger;

#[no_mangle]
pub extern "C" fn main() {
    unsafe { log::set_logger_racy(&XalLogger) };
    log::set_max_level(log::LevelFilter::Warn);
    info!("Echo client main");
    let partition = PeriodicEchoPartition::new(
        unsafe { &SENDER },
        unsafe { &RECEIVER },
        entry_point_periodic,
        entry_point_aperiodic,
    );
    partition.run()
}

extern "C" fn entry_point_periodic() {
    EchoSenderProcess::run(unsafe { SENDER.get_mut().unwrap() });
}

extern "C" fn entry_point_aperiodic() {
    EchoReceiverProcess::run(unsafe { RECEIVER.get_mut().unwrap() });
}
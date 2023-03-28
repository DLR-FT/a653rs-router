#![no_std]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use apex_rs::prelude::*;
use apex_rs_xng::apex::XngHypervisor;

#[cfg(feature = "sender")]
use apex_throughput::TrafficSender;

#[cfg(feature = "receiver")]
use apex_throughput::{TrafficReceiver, TrafficReceiverPartition};

use coraz7::XalLogger;
use core::time::Duration;
use once_cell::unsync::OnceCell;

// TODO make configurable
// Maximum from XNG header files
const MSG: MessageSize = 8192;
const FIFO: MessageRange = 256;
// As defined in LithOS constraints
const INTERVAL: Duration = Duration::from_micros(500);
type Hypervisor = XngHypervisor;

static LOGGER: XalLogger = XalLogger;

fn setup_logger() {
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(log::LevelFilter::Info);
}

#[cfg(feature = "sender")]
static mut SENDER: OnceCell<QueuingPortSender<MSG, FIFO, Hypervisor>> = OnceCell::new();

#[cfg(feature = "sender")]
#[no_mangle]
pub extern "C" fn main() {
    setup_logger();
    let part = unsafe { TrafficSender::new(&SENDER, sender, SystemTime::Normal(INTERVAL)) };
    part.run();
}

#[cfg(feature = "sender")]
#[no_mangle]
pub extern "C" fn sender() {
    apex_throughput::send::<MSG, FIFO, Hypervisor>(
        unsafe { SENDER.get_mut().unwrap() },
        &SystemTime::Normal(INTERVAL),
    );
}

#[cfg(feature = "receiver")]
static mut RECEIVER_PORT: OnceCell<QueuingPortReceiver<MSG, FIFO, Hypervisor>> = OnceCell::new();

#[cfg(feature = "receiver")]
#[no_mangle]
pub extern "C" fn main() {
    setup_logger();
    let part = unsafe { TrafficReceiverPartition::new(&RECEIVER_PORT, receiver, logger) };
    part.run();
}

#[cfg(feature = "receiver")]
static mut RECEIVER: TrafficReceiver<MSG, FIFO> = TrafficReceiver::new();

#[cfg(feature = "receiver")]
#[no_mangle]
pub extern "C" fn receiver() {
    unsafe {
        RECEIVER.receive::<Hypervisor>(
            RECEIVER_PORT.get_mut().unwrap(),
            &SystemTime::Normal(INTERVAL),
        )
    };
}

#[cfg(feature = "receiver")]
#[no_mangle]
pub extern "C" fn logger() {
    unsafe { RECEIVER.log::<Hypervisor>() }
}

#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_rs::prelude::ApexTimeP1Ext;
use apex_rs_xng::apex::XngHypervisor;
use log::{Level, LevelFilter, Metadata, Record};
use network_partition_xng::{network::UartSerial, logging::XalLogger};
use log;

include!(concat!(env!("OUT_DIR"), "/np.rs"));

static LOGGER: XalLogger = XalLogger;

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn main() {
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(log::Level::Trace);
    NetworkPartition.run();
}

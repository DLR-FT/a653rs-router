#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use a653rs::prelude::*;
use a653rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use apex_echo::server::*;
use log::LevelFilter;
use once_cell::unsync::OnceCell;

const ECHO_SIZE: MessageSize = 1000;

static mut RECEIVER: OnceCell<SamplingPortDestination<ECHO_SIZE, ApexLinuxPartition>> =
    OnceCell::new();
static mut SENDER: OnceCell<SamplingPortSource<ECHO_SIZE, ApexLinuxPartition>> = OnceCell::new();

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Info).unwrap();
    let partition = EchoServerPartition::new(
        unsafe { &SENDER },
        unsafe { &RECEIVER },
        entry_point_periodic,
    );
    partition.run()
}

extern "C" fn entry_point_periodic() {
    EchoServerProcess::run(unsafe { SENDER.get_mut().unwrap() }, unsafe {
        RECEIVER.get_mut().unwrap()
    })
}

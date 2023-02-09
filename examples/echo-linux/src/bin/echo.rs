#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use apex_echo::client::*;
use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use log::LevelFilter;
use once_cell::unsync::OnceCell;

const ECHO_SIZE: MessageSize = 1000;

static mut SENDER: OnceCell<SamplingPortSource<ECHO_SIZE, ApexLinuxPartition>> = OnceCell::new();
static mut RECEIVER: OnceCell<SamplingPortDestination<ECHO_SIZE, ApexLinuxPartition>> =
    OnceCell::new();

fn main() {
    // Register panic info print on panic
    ApexLogger::install_panic_hook();

    // Log all events down to trace level
    ApexLogger::install_logger(LevelFilter::Info).unwrap();

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

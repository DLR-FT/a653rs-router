extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use log::LevelFilter;
use network_partition::prelude::*;
use once_cell::sync::OnceCell;

const ECHO_PORT_SIZE_BYTES: u32 = 10000;

static ECHO_SEND: OnceCell<SamplingPortSource<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>> =
    OnceCell::new();

static ECHO_RECV: OnceCell<SamplingPortDestination<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>> =
    OnceCell::new();

fn main() {
    // Register panic info print on panic
    ApexLogger::install_panic_hook();

    // Log all events down to trace level
    ApexLogger::install_logger(LevelFilter::Info).unwrap();

    let partition =
        NetworkPartition::<ApexLinuxPartition>::new(&ECHO_RECV, &ECHO_SEND, entry_point);
    partition.run()
}

extern "C" fn entry_point() {
    let input = ECHO_RECV.get().unwrap();
    let output = ECHO_SEND.get().unwrap();
    loop {
        _ = input.forward(&output);
        ApexLinuxPartition::periodic_wait().unwrap();
    }
}

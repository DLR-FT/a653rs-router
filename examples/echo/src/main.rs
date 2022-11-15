use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};

use echo::{EchoClient, EchoPartition, EchoReceiver, EchoSender};
use log::LevelFilter;
use once_cell::sync::{Lazy, OnceCell};
use std::time::Duration;

const ECHO_PORT_SIZE_BYTES: u32 = 1000;

static CLIENT: OnceCell<EchoClient<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>> = OnceCell::new();

static PERIOD: Lazy<Duration> = Lazy::new(|| {
    EchoPartition::<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>::get_partition_status()
        .period
        .unwrap_duration()
});

pub extern "C" fn periodic_send() {
    CLIENT.get().unwrap().send();
}

pub extern "C" fn aperiodic_receive() {
    CLIENT.get().unwrap().receive();
}

fn main() {
    // Register panic info print on panic
    ApexLogger::install_panic_hook();

    // Log all events down to trace level
    ApexLogger::install_logger(LevelFilter::Info).unwrap();

    _ = CLIENT.set(EchoClient::<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition> {
        sender: OnceCell::new(),
        receiver: OnceCell::new(),
        entry_point_periodic: periodic_send,
        entry_point_aperiodic: aperiodic_receive,
        echo_validity: PERIOD.checked_mul(2).unwrap(),
    });

    let partition = EchoPartition {
        client: &CLIENT.get().unwrap(),
    };
    partition.run()
}

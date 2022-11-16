#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

mod echo;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use echo::*;
use log::LevelFilter;
use once_cell::sync::{Lazy, OnceCell};
use std::time::Duration;

const ECHO_SIZE: MessageSize = 1000;

static PERIOD: Lazy<Duration> = Lazy::new(|| {
    PeriodicEchoPartition::<ECHO_SIZE, ApexLinuxPartition>::get_partition_status()
        .period
        .unwrap_duration()
});

static SENDER: OnceCell<SamplingPortSource<ECHO_SIZE, ApexLinuxPartition>> = OnceCell::new();

static RECEIVER: OnceCell<SamplingPortDestination<ECHO_SIZE, ApexLinuxPartition>> = OnceCell::new();

fn main() {
    // Register panic info print on panic
    ApexLogger::install_panic_hook();

    // Log all events down to trace level
    ApexLogger::install_logger(LevelFilter::Info).unwrap();

    let echo_validity: Duration = PERIOD.checked_mul(2).unwrap();
    let partition = PeriodicEchoPartition::<ECHO_SIZE, ApexLinuxPartition>::new(
        echo_validity,
        &SENDER,
        &RECEIVER,
        entry_point_periodic,
        entry_point_aperiodic,
    );
    partition.run()
}

extern "C" fn entry_point_periodic() {
    SENDER.get().unwrap().run_process();
}

extern "C" fn entry_point_aperiodic() {
    RECEIVER.get().unwrap().run_process();
}

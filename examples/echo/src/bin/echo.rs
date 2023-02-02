#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use apex_rs_postcard::sampling::{SamplingPortDestinationExt, SamplingPortSourceExt};
use core::hint;
use core::str::FromStr;
use core::time::Duration;
use log::LevelFilter;
use log::{error, info, trace};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
/// Echo message
struct Echo {
    /// A sequence number.
    pub sequence: u32,

    /// The time at which the message has been created.
    pub when_ms: u64,
}

pub trait RunableProcess {
    // TODO should take ownership of port, but can't take ownership of port as part of static lifetime process
    /// Run the process
    fn run_process(&self) -> !;
}

impl<const ECHO_SIZE: MessageSize, H> RunableProcess for SamplingPortSource<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4 + ApexTimeP4Ext,
    [u8; ECHO_SIZE as usize]:,
{
    fn run_process(&self) -> ! {
        let mut i: u32 = 0;
        loop {
            i += 1;
            let now = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
            let data = Echo {
                sequence: i,
                when_ms: now.as_millis() as u64,
            };
            let result = self.send_type(data);
            match result {
                Ok(_) => {
                    trace!(
                        "EchoRequest: seqnr = {:?}, time = {:?}",
                        data.sequence,
                        data.when_ms
                    );
                }
                Err(_) => {
                    error!("Failed to send EchoRequest");
                }
            }
            <H as ApexTimeP4Ext>::periodic_wait().unwrap();
        }
    }
}

impl<const ECHO_SIZE: MessageSize, H> RunableProcess for SamplingPortDestination<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4 + ApexTimeP4Ext,
    [u8; ECHO_SIZE as usize]:,
{
    fn run_process(&self) -> ! {
        let mut last = 0;
        loop {
            let now = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
            let result = self.recv_type::<Echo>();
            match result {
                Ok(data) => {
                    let (valid, received) = data;
                    if received.sequence > last {
                        last = received.sequence;
                        info!(
                            "EchoReply: seqnr = {:?}, time = {:?}, valid = {valid:?}",
                            received.sequence,
                            (now.as_millis() as u64) - received.when_ms
                        );
                    }
                }
                Err(_) => {
                    trace!("Failed to receive anything");
                }
            }
            for _ in 0..10000 {
                hint::spin_loop()
            }
        }
    }
}

pub struct PeriodicEchoPartition<const ECHO_SIZE: MessageSize, S>
where
    S: ApexSamplingPortP4 + 'static,
{
    sender: &'static OnceCell<SamplingPortSource<ECHO_SIZE, S>>,
    receiver: &'static OnceCell<SamplingPortDestination<ECHO_SIZE, S>>,
    entry_point_periodic: extern "C" fn(),
    entry_point_aperiodic: extern "C" fn(),
    echo_validity: Duration,
}

impl<const ECHO_SIZE: MessageSize, H> Partition<H> for PeriodicEchoPartition<ECHO_SIZE, H>
where
    H: ApexPartitionP4 + ApexProcessP4 + ApexSamplingPortP4,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        let send_port = ctx
            .create_sampling_port_source(Name::from_str("EchoRequest").unwrap())
            .unwrap();

        _ = self.sender.set(send_port);

        let receive_port = ctx
            .create_sampling_port_destination(
                Name::from_str("EchoReply").unwrap(),
                self.echo_validity,
            )
            .unwrap();
        _ = self.receiver.set(receive_port);

        // Periodic
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::ZERO),
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point_periodic,
            stack_size: 100000,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("periodic_echo_send").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap();

        // Aperiodic
        ctx.create_process(ProcessAttribute {
            // There can be only one process with normal period
            period: SystemTime::Infinite,
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point_aperiodic,
            stack_size: 100000,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("aperiodic_echo_receive").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap()
    }

    fn warm_start(&self, ctx: &mut StartContext<H>) {
        self.cold_start(ctx)
    }
}

impl<const ECHO_SIZE: MessageSize, H> PeriodicEchoPartition<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4,
    [u8; ECHO_SIZE as usize]:,
{
    pub fn new(
        echo_validity: Duration,
        sender: &'static OnceCell<SamplingPortSource<ECHO_SIZE, H>>,
        receiver: &'static OnceCell<SamplingPortDestination<ECHO_SIZE, H>>,
        entry_point_periodic: extern "C" fn(),
        entry_point_aperiodic: extern "C" fn(),
    ) -> Self {
        PeriodicEchoPartition::<ECHO_SIZE, H> {
            sender,
            receiver,
            entry_point_periodic,
            entry_point_aperiodic,
            echo_validity,
        }
    }
}
extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use apex_rs_postcard::prelude::*;
use log::{info, trace, LevelFilter};
use network_partition::prelude::*;
use once_cell::sync::OnceCell;
use std::str::FromStr; // Name::from_str
use std::thread::sleep;
use std::time::Duration;

pub type Hypervisor = ApexLinuxPartition;

struct EchoSender;

trait PartitionName {
    fn name(&self) -> Name;
}

impl PartitionName for EchoSender {
    fn name(&self) -> Name {
        Name::from_str("Echo").unwrap()
    }
}

const ECHO_PORT_SIZE_BYTES: u32 = 1000;

static ECHO_SEND: OnceCell<SamplingPortSource<ECHO_PORT_SIZE_BYTES, Hypervisor>> = OnceCell::new();

static ECHO_RECV: OnceCell<SamplingPortDestination<ECHO_PORT_SIZE_BYTES, Hypervisor>> =
    OnceCell::new();

impl Partition<Hypervisor> for EchoSender {
    fn cold_start(&self, ctx: &mut StartContext<Hypervisor>) {
        let send_port = ctx
            .create_sampling_port_source(Name::from_str("EchoRequest").unwrap())
            .unwrap();
        ECHO_SEND.set(send_port).unwrap();

        let receive_port = ctx
            .create_sampling_port_destination(
                Name::from_str("EchoReply").unwrap(),
                Duration::from_millis(1),
            )
            .unwrap();
        ECHO_RECV.set(receive_port).unwrap();

        ctx.create_process(ProcessAttribute {
            // Never repeat
            period: SystemTime::Normal(Duration::ZERO),

            // May run forever
            time_capacity: SystemTime::Infinite,

            // Send some ECHO probes
            entry_point: periodic_echo,

            stack_size: 100000,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("periodic_echo").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap()
    }

    fn warm_start(&self, ctx: &mut StartContext<Hypervisor>) {
        self.cold_start(ctx)
    }
}

extern "C" fn periodic_echo() {
    for i in 1..i32::MAX {
        sleep(Duration::from_millis(1));
        let now = Hypervisor::get_time().unwrap_duration().as_millis() as u64;
        let data = Echo {
            sequence: i,
            when_ms: now,
        };
        ECHO_SEND.get().unwrap().send_type(data).ok().unwrap();

        let recv_now = Hypervisor::get_time().unwrap_duration().as_millis() as u64;
        let result = ECHO_RECV.get().unwrap().recv_type::<Echo>();
        match result {
            Ok(data) => {
                let (valid, received) = data;
                info!(
                    "EchoReply: seqnr = {:?}, time = {:?}, valid: {valid:?}",
                    received.sequence,
                    recv_now - received.when_ms
                );
            }
            Err(_) => {
                trace!("No echo reply")
            }
        }

        Hypervisor::periodic_wait().unwrap();
    }
}

fn main() {
    // Register panic info print on panic
    ApexLogger::install_panic_hook();

    // Log all events down to trace level
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();

    EchoSender.run()
}

extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use apex_rs_postcard::prelude::*;
use log::{trace, LevelFilter};
use network_partition::prelude::*;
use once_cell::sync::OnceCell;
use std::str::FromStr; // Name::from_str
use std::thread::sleep;
use std::time::Duration;

pub type Hypervisor = ApexLinuxPartition;

struct NetworkPartition;

// TODO move to lib
trait PartitionName {
    fn name(&self) -> Name;
}

impl PartitionName for NetworkPartition {
    fn name(&self) -> Name {
        Name::from_str("NetworkPartition").unwrap()
    }
}

const ECHO_PORT_SIZE_BYTES: u32 = 1000;

static ECHO_SEND: OnceCell<SamplingPortSource<ECHO_PORT_SIZE_BYTES, Hypervisor>> = OnceCell::new();

static ECHO_RECV: OnceCell<SamplingPortDestination<ECHO_PORT_SIZE_BYTES, Hypervisor>> =
    OnceCell::new();

impl Partition<Hypervisor> for NetworkPartition {
    fn cold_start(&self, ctx: &mut StartContext<Hypervisor>) {
        let receive_port = ctx
            .create_sampling_port_destination(
                Name::from_str("EchoRequest").unwrap(),
                Duration::from_millis(1),
            )
            .unwrap();
        ECHO_RECV.set(receive_port).unwrap();

        let send_port = ctx
            .create_sampling_port_source(Name::from_str("EchoReply").unwrap())
            .unwrap();
        ECHO_SEND.set(send_port).unwrap();

        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::ZERO),
            time_capacity: SystemTime::Infinite,
            entry_point: respond_to_echo,
            stack_size: 100000,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("respond_to_echo").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap()
    }

    fn warm_start(&self, ctx: &mut StartContext<Hypervisor>) {
        self.cold_start(ctx)
    }
}

extern "C" fn respond_to_echo() {
    for _ in 1..i32::MAX {
        sleep(Duration::from_millis(1));
        let result = ECHO_RECV.get().unwrap().recv_type::<Echo>();
        match result {
            Ok(data) => {
                let (valid, data) = data;
                trace!("Echo seqnr: {:?}, valid: {valid:?}", data.sequence);
                ECHO_SEND.get().unwrap().send_type(data).ok().unwrap();
            }
            Err(_) => trace!("No echo request"),
        }

        Hypervisor::periodic_wait().unwrap();
    }
}

fn main() {
    // Register panic info print on panic
    ApexLogger::install_panic_hook();

    // Log all events down to trace level
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();

    NetworkPartition.run()
}

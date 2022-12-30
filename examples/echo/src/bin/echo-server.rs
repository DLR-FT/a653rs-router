#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use core::str::FromStr;
use core::time::Duration;
use log::{error, trace, LevelFilter};
use once_cell::sync::OnceCell;

const ECHO_SIZE: MessageSize = 1000;

static RECEIVER: OnceCell<SamplingPortDestination<ECHO_SIZE, ApexLinuxPartition>> = OnceCell::new();
static SENDER: OnceCell<SamplingPortSource<ECHO_SIZE, ApexLinuxPartition>> = OnceCell::new();

type Hypervisor = ApexLinuxPartition;

struct EchoServer;

impl Partition<Hypervisor> for EchoServer {
    fn cold_start(&self, ctx: &mut StartContext<Hypervisor>) {
        {
            let recv = ctx
                .create_sampling_port_destination(
                    Name::from_str("EchoRequest").unwrap(),
                    Duration::from_secs(1),
                )
                .unwrap();
            RECEIVER.set(recv).unwrap();
        };

        {
            let send = ctx
                .create_sampling_port_source(Name::from_str("EchoReply").unwrap())
                .unwrap();
            SENDER.set(send).unwrap();
        };

        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::ZERO),
            time_capacity: SystemTime::Infinite,
            entry_point: entry_point_periodic,
            stack_size: 100000,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("echo_server").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap();
    }

    fn warm_start(&self, ctx: &mut StartContext<Hypervisor>) {
        self.cold_start(ctx)
    }
}

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Info).unwrap();
    EchoServer.run()
}

extern "C" fn entry_point_periodic() {
    let send = SENDER.get().unwrap();
    let recv = RECEIVER.get().unwrap();
    let mut buf = [0u8; ECHO_SIZE as usize];
    loop {
        match recv.receive(&mut buf) {
            Ok((val, data)) => {
                if val == Validity::Valid {
                    match send.send(data) {
                        Ok(_) => {
                            trace!("Replied to echo");
                        }
                        Err(err) => {
                            error!("Failed to reply to echo: {:?}", err);
                        }
                    }
                } else {
                    trace!("Ignoring invalid data");
                }
            }
            _ => {
                error!("Failed to receive echo");
            }
        }
        Hypervisor::periodic_wait().unwrap();
    }
}

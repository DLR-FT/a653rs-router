use apex_rs::prelude::*;
use apex_rs_postcard::{
    prelude::SamplingRecvError,
    sampling::{SamplingPortDestinationExt, SamplingPortSourceExt},
};
use core::str::FromStr;
use core::time::Duration;
use log::{error, info, trace, warn};
use once_cell::unsync::OnceCell;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
/// Echo message
struct Echo {
    /// A sequence number.
    pub sequence: u32,

    /// The time at which the message has been created.
    pub when_ms: u64,
}

#[derive(Debug)]
pub struct EchoSenderProcess<const ECHO_SIZE: MessageSize>;

impl<const ECHO_SIZE: MessageSize> EchoSenderProcess<ECHO_SIZE>
where
    [u8; ECHO_SIZE as usize]:,
{
    pub fn run<H: ApexSamplingPortP4 + ApexTimeP4Ext>(
        port: &mut SamplingPortSource<ECHO_SIZE, H>,
    ) -> ! {
        info!("Running echo client periodic process");
        let mut i: u32 = 0;
        loop {
            i += 1;
            let now = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
            let data = Echo {
                sequence: i,
                when_ms: now.as_millis() as u64,
            };
            let result = port.send_type(data);
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

#[derive(Debug)]
pub struct EchoReceiverProcess<const ECHO_SIZE: MessageSize>;

impl<const ECHO_SIZE: MessageSize> EchoReceiverProcess<ECHO_SIZE>
where
    [u8; ECHO_SIZE as usize]:,
{
    pub fn run<H: ApexSamplingPortP4 + ApexTimeP4Ext>(
        port: &mut SamplingPortDestination<ECHO_SIZE, H>,
    ) -> ! {
        let mut last = 0;
        loop {
            trace!("Running echo client aperiodic process");
            let now = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
            let result = port.recv_type::<Echo>();
            match result {
                Ok(data) => {
                    let (valid, received) = data;
                    // Reset when client restarts
                    if received.sequence == 1 {
                        last = 0;
                    }
                    if received.sequence > last {
                        last = received.sequence;
                        info!(
                            "EchoReply: seqnr = {:?}, time = {:?}, valid = {valid:?}",
                            received.sequence,
                            (now.as_millis() as u64) - received.when_ms
                        );
                    } else {
                        trace!("Duplicate")
                    }
                }
                Err(SamplingRecvError::Apex(Error::NotAvailable)) => {
                    trace!("No echo reply available");
                }
                Err(e) => {
                    error!("Failed to receive reply: {e:?}");
                }
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
}

impl<const ECHO_SIZE: MessageSize, H> Partition<H> for PeriodicEchoPartition<ECHO_SIZE, H>
where
    H: ApexPartitionP4 + ApexProcessP4 + ApexSamplingPortP4,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        trace!("Cold start echo client");
        let send_port = ctx
            .create_sampling_port_source(Name::from_str("EchoRequest").unwrap())
            .unwrap();

        _ = self.sender.set(send_port);

        let echo_validity: Duration = Duration::from_secs(10);

        let receive_port = ctx
            .create_sampling_port_destination(Name::from_str("EchoReply").unwrap(), echo_validity)
            .unwrap();
        _ = self.receiver.set(receive_port);

        // Periodic
        trace!("Creating periodic echo process");
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::from_secs(1)),
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point_periodic,
            stack_size: 10000,
            base_priority: 5,
            deadline: Deadline::Soft,
            name: Name::from_str("EchoSend").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap();

        // Aperiodic
        trace!("Creating aperiodic echo process");
        ctx.create_process(ProcessAttribute {
            // There can be only one process with normal period
            period: SystemTime::Infinite,
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point_aperiodic,
            stack_size: 10000,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("EchoReceive").unwrap(),
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
        }
    }
}

use apex_rs::prelude::*;
use apex_rs_postcard::sampling::{SamplingPortDestinationExt, SamplingPortSourceExt};
use log::{error, info, trace};
use network_partition::echo::Echo;
use once_cell::sync::OnceCell;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

pub trait EchoProcess {
    // TODO should take ownership of port, but can't take ownership of port as part of static lifetime process
    fn run(&self) -> !;
}

pub struct SendPeriodicEchoProcess<const ECHO_SIZE: MessageSize, S>
where
    S: ApexSamplingPortP4,
{
    sender: SamplingPortSource<ECHO_SIZE, S>,
}

impl<const ECHO_SIZE: MessageSize, H> EchoProcess for SendPeriodicEchoProcess<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4 + ApexTimeP4Ext,
    [u8; ECHO_SIZE as usize]:,
{
    fn run(&self) -> ! {
        let mut i: u32 = 0;
        loop {
            i = i + 1;
            let now = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
            let data = Echo {
                sequence: i,
                when_ms: now.as_millis() as u64,
            };
            let result = self.sender.send_type(data);
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

pub struct ReceivePeriodicEchoProcess<const ECHO_SIZE: MessageSize, S>
where
    S: ApexSamplingPortP4,
{
    receiver: SamplingPortDestination<ECHO_SIZE, S>,
}

impl<const ECHO_SIZE: MessageSize, H> EchoProcess for ReceivePeriodicEchoProcess<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4 + ApexTimeP4Ext,
    [u8; ECHO_SIZE as usize]:,
{
    fn run(&self) -> ! {
        loop {
            let now = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
            let result = self.receiver.recv_type::<Echo>();
            match result {
                Ok(data) => {
                    let (valid, received) = data;
                    info!(
                        "EchoReply: seqnr = {:?}, time = {:?}, valid = {valid:?}",
                        received.sequence,
                        (now.as_millis() as u64) - received.when_ms
                    );
                }
                Err(_) => {
                    trace!("Failed to receive anything");
                }
            }
            sleep(Duration::from_millis(20));
        }
    }
}

pub struct PeriodicEchoPartition<const ECHO_SIZE: MessageSize, S>
where
    S: ApexSamplingPortP4 + 'static,
{
    sender: &'static OnceCell<SendPeriodicEchoProcess<ECHO_SIZE, S>>,
    receiver: &'static OnceCell<ReceivePeriodicEchoProcess<ECHO_SIZE, S>>,
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

        _ = self
            .sender
            .set(SendPeriodicEchoProcess { sender: send_port });

        let receive_port = ctx
            .create_sampling_port_destination(
                Name::from_str("EchoReply").unwrap(),
                self.echo_validity,
            )
            .unwrap();
        _ = self.receiver.set(ReceivePeriodicEchoProcess {
            receiver: receive_port,
        });

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

pub trait EchoPartition<const ECHO_SIZE: MessageSize, H>
where
    H: ApexSamplingPortP4,
{
    fn new(
        echo_validity: Duration,
        sender: &'static OnceCell<SendPeriodicEchoProcess<ECHO_SIZE, H>>,
        receiver: &'static OnceCell<ReceivePeriodicEchoProcess<ECHO_SIZE, H>>,
        entry_point_periodic: extern "C" fn(),  //SystemAddr,
        entry_point_aperiodic: extern "C" fn(), //SystemAddr
    ) -> Self;
}

impl<const ECHO_SIZE: MessageSize, H> EchoPartition<ECHO_SIZE, H>
    for PeriodicEchoPartition<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4,
    [u8; ECHO_SIZE as usize]:,
{
    fn new(
        echo_validity: Duration,
        sender: &'static OnceCell<SendPeriodicEchoProcess<ECHO_SIZE, H>>,
        receiver: &'static OnceCell<ReceivePeriodicEchoProcess<ECHO_SIZE, H>>,
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

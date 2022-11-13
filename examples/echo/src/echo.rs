use apex_rs::prelude::*;
use apex_rs_postcard::sampling::{SamplingPortDestinationExt, SamplingPortSourceExt};
use log::{error, info, trace};
use network_partition::echo::Echo;
use once_cell::sync::OnceCell;
use std::str::FromStr;
use std::time::Duration;

pub struct EchoPartition<'a, const MSG_SIZE: MessageSize, S>
where
    S: ApexSamplingPortP4,
{
    pub client: &'a EchoClient<MSG_SIZE, S>,
}

pub struct EchoClient<const MSG_SIZE: MessageSize, S>
where
    S: ApexSamplingPortP4,
{
    pub sender: OnceCell<SamplingPortSource<MSG_SIZE, S>>,
    pub receiver: OnceCell<SamplingPortDestination<MSG_SIZE, S>>,
    pub entry_point_periodic: extern "C" fn(),
    pub entry_point_aperiodic: extern "C" fn(),
    pub echo_validity: Duration,
}

impl<'a, const MSG_SIZE: MessageSize, H> Partition<H> for EchoPartition<'a, MSG_SIZE, H>
where
    H: ApexPartitionP4 + ApexProcessP4 + ApexSamplingPortP4 + 'a,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        let send_port = ctx
            .create_sampling_port_source(Name::from_str("EchoRequest").unwrap())
            .unwrap();
        _ = self.client.sender.set(send_port);

        let receive_port = ctx
            .create_sampling_port_destination(
                Name::from_str("EchoReply").unwrap(),
                self.client.echo_validity,
            )
            .unwrap();
        _ = self.client.receiver.set(receive_port);

        // Periodic
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::ZERO),
            time_capacity: SystemTime::Infinite,
            entry_point: self.client.entry_point_periodic,
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
            entry_point: self.client.entry_point_aperiodic,
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

pub trait EchoSender {
    fn send(&self, i: u32);
}

pub trait EchoReceiver {
    fn receive(&self);
}

impl<const MSG_SIZE: MessageSize, S> EchoSender for EchoClient<MSG_SIZE, S>
where
    S: ApexSamplingPortP4 + ApexTimeP4Ext,
    [u8; MSG_SIZE as usize]:,
{
    fn send(&self, i: u32) {
        let now = <S as ApexTimeP4Ext>::get_time().unwrap_duration();
        let data = Echo {
            sequence: i,
            when_ms: now.as_millis() as u64,
        };
        let result = self.sender.get().unwrap().send_type(data);
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
    }
}

impl<const MSG_SIZE: MessageSize, S> EchoReceiver for EchoClient<MSG_SIZE, S>
where
    S: ApexSamplingPortP4 + ApexTimeP4Ext,
    [u8; MSG_SIZE as usize]:,
{
    fn receive(&self) {
        let now = <S as ApexTimeP4Ext>::get_time().unwrap_duration();
        let receiver = self.receiver.get().unwrap();
        let result = receiver.recv_type::<Echo>();
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
    }
}

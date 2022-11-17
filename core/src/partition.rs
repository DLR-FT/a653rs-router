use crate::config::Config;
use crate::echo::PortSampler;
use apex_rs::prelude::*;
use core::str::FromStr;
use once_cell::sync::OnceCell;
use std::time::Duration;

type SystemAddress = extern "C" fn();

/// NetworkPartition that processes the ports in sequence and performs
/// registered actions on them.
/// loop
///   sample_each_sampling_port_destination
///     match data type / port name
///       perform registered actions for match
/// TODO must be able to iterate over all destinations
#[derive(Debug)]
pub struct NetworkPartition<const ECHO_SIZE: MessageSize, H>
where
    H: ApexSamplingPortP4 + 'static,
{
    config: Config,
    echo_destination: &'static OnceCell<SamplingPortDestination<ECHO_SIZE, H>>,
    echo_source: &'static OnceCell<SamplingPortSource<ECHO_SIZE, H>>,
    entry_point: SystemAddress,
}

impl<const ECHO_SIZE: MessageSize, H> NetworkPartition<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4,
{
    /// Create a new instance of the network partition
    pub fn new(
        config: Config,
        echo_destination: &'static OnceCell<SamplingPortDestination<ECHO_SIZE, H>>,
        echo_source: &'static OnceCell<SamplingPortSource<ECHO_SIZE, H>>,
        entry_point: SystemAddress,
    ) -> Self {
        NetworkPartition::<ECHO_SIZE, H> {
            config,
            echo_destination,
            echo_source,
            entry_point,
        }
    }
}

impl<const ECHO_SIZE: MessageSize, H> Partition<H> for NetworkPartition<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        let receive_port = ctx
            .create_sampling_port_destination(
                Name::from_str("EchoRequest").unwrap(),
                Duration::from_millis(100000), // TODO make configurable
            )
            .unwrap();
        _ = self.echo_destination.set(receive_port);

        let send_port = ctx
            .create_sampling_port_source(Name::from_str("EchoReply").unwrap())
            .unwrap();
        _ = self.echo_source.set(send_port);

        // Periodic
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::ZERO),
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point,
            stack_size: self.config.stack_size.periodic_process.as_u64() as u32,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("respond_to_echo").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap()
    }

    fn warm_start(&self, ctx: &mut StartContext<H>) {
        self.cold_start(ctx)
    }
}

/// Runs the main loop of the network partition.
pub fn run<const MSG_SIZE: MessageSize, H>(
    input: &SamplingPortDestination<MSG_SIZE, H>,
    output: &SamplingPortSource<MSG_SIZE, H>,
) -> !
where
    H: ApexSamplingPortP4 + ApexTimeP4Ext,
    [u8; MSG_SIZE as usize]:,
{
    loop {
        _ = input.forward(&output);
        <H as ApexTimeP4Ext>::periodic_wait().unwrap();
    }
}

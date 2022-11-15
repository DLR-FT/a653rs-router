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
#[derive(Debug)]
pub struct NetworkPartition<H>
where
    H: ApexSamplingPortP4 + 'static,
{
    // TODO must be able to iterate over all destinations
    destination: &'static OnceCell<SamplingPortDestination<10000, H>>,
    source: &'static OnceCell<SamplingPortSource<10000, H>>,
    entry_point: SystemAddress,
}

impl<H> NetworkPartition<H>
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4,
{
    /// Create a new instance of the network partition
    pub fn new(
        destination: &'static OnceCell<SamplingPortDestination<10000, H>>,
        source: &'static OnceCell<SamplingPortSource<10000, H>>,
        entry_point: SystemAddress,
    ) -> Self {
        NetworkPartition::<H> {
            destination,
            source,
            entry_point,
        }
    }
}

impl<H> Partition<H> for NetworkPartition<H>
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        let receive_port = ctx
            .create_sampling_port_destination(
                Name::from_str("EchoRequest").unwrap(),
                Duration::from_millis(100000),
            )
            .unwrap();
        _ = self.destination.set(receive_port);

        let send_port = ctx
            .create_sampling_port_source(Name::from_str("EchoReply").unwrap())
            .unwrap();
        _ = self.source.set(send_port);

        // Periodic
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::ZERO),
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point,
            stack_size: 100000,
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

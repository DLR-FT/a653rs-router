use crate::config::Config;
use crate::echo::PortSampler;
use apex_rs::prelude::*;
use core::str::FromStr;
use log::{error, trace};
use once_cell::sync::OnceCell;
use std::time::Duration;

// TODO platform-specific. read from environment?
static CONFIG: &'static str = include_str!("../../config/network_partition_config.yml");

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
    echo_destination: &'static OnceCell<SamplingPortDestination<10000, H>>,
    echo_source: &'static OnceCell<SamplingPortSource<10000, H>>,
    entry_point: SystemAddress,
}

impl<H> NetworkPartition<H>
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4,
{
    /// Create a new instance of the network partition
    pub fn new(
        echo_destination: &'static OnceCell<SamplingPortDestination<10000, H>>,
        echo_source: &'static OnceCell<SamplingPortSource<10000, H>>,
        entry_point: SystemAddress,
    ) -> Self {
        NetworkPartition::<H> {
            echo_destination,
            echo_source,
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
            stack_size: 100000, // TODO make configurable
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
    let parsed_config = serde_yaml::from_str::<Config>(CONFIG);
    if let Err(error) = parsed_config {
        error!("{error:?}");
        panic!();
    }
    let config = parsed_config.ok().unwrap();
    trace!("Have config: {config:?}");

    loop {
        _ = input.forward(&output);
        <H as ApexTimeP4Ext>::periodic_wait().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::CONFIG;
    use crate::config::Config;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn parse_code_section_config() {
        let parsed = serde_yaml::from_str::<Config>(CONFIG);
        assert!(parsed.is_ok());
    }
}

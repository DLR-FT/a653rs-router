use crate::config::*;
use crate::echo::PortSampler;
use crate::network::VirtualLink;
use crate::ports::ChannelName;
use crate::routing::{Router, RouterP4};
use apex_rs::prelude::*;
use core::str::FromStr;
use once_cell::sync::OnceCell;
use std::fmt::Debug;
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
pub struct NetworkPartition<const ECHO_SIZE: MessageSize, H: ApexSamplingPortP4 + 'static> {
    config: Config,
    router: &'static OnceCell<RouterP4<ECHO_SIZE, H>>,
    entry_point: SystemAddress,
}

impl<const ECHO_SIZE: MessageSize, H> NetworkPartition<ECHO_SIZE, H>
where
    H: ApexSamplingPortP4,
{
    /// Create a new instance of the network partition
    pub fn new(
        config: Config,
        router: &'static OnceCell<RouterP4<ECHO_SIZE, H>>,
        entry_point: SystemAddress,
    ) -> Self {
        NetworkPartition::<ECHO_SIZE, H> {
            config,
            router,
            entry_point,
        }
    }
}

// TODO create all ports and processes from config
impl<const MSG_SIZE: MessageSize, H> Partition<H> for NetworkPartition<MSG_SIZE, H>
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4 + Debug,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        let mut router = RouterP4::<MSG_SIZE, H>::new();
        let echo_request = ChannelName::from_str("EchoRequest").unwrap();
        let echo_reply = ChannelName::from_str("EchoReply").unwrap();

        // Cannot dynamically init ports with values from config because message sizes are not known at compile time
        // Maybe code generation could be used to translate the config into code -> const values -> can be used in generics
        let echo_request_port_config = self
            .config
            .ports
            .clone()
            .into_iter()
            .filter_map(|x| {
                let config: SamplingPortDestinationConfig = x.sampling_port_destination()?;
                if config.channel == "EchoRequest" {
                    Some(config)
                } else {
                    None
                }
            })
            .last();

        if let Some(config) = echo_request_port_config {
            let name = Name::from_str("EchoRequest").unwrap();
            let port = ctx
                .create_sampling_port_destination::<MSG_SIZE>(name, config.validity)
                .unwrap();
            router.add_local_destination(echo_request.clone(), port);
        }

        let echo_reply_port_config = self
            .config
            .ports
            .clone()
            .into_iter()
            .filter_map(|x| {
                let config: SamplingPortSourceConfig = x.sampling_port_source()?;
                if config.channel == "EchoReply" {
                    Some(config)
                } else {
                    None
                }
            })
            .last();

        if let Some(_) = echo_reply_port_config {
            let port = ctx
                .create_sampling_port_source::<MSG_SIZE>(echo_reply.clone().into_inner())
                .unwrap();

            router.add_local_source(echo_reply.clone(), port);
            router.add_output_route(echo_request.clone(), 0).unwrap();
            router.add_input_route(0, echo_reply.clone()).unwrap();
        }

        // TODO use config properly
        self.router.set(router).unwrap();

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

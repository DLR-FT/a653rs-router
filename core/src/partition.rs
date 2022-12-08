use crate::config::*;
use crate::ports::ChannelId;
use crate::routing::Router;
use crate::virtual_link::VirtualLinkId;
use apex_rs::prelude::*;
use core::fmt::Debug;
use core::str::FromStr;
use core::time::Duration;
use heapless::LinearMap;
use once_cell::sync::OnceCell;

type SystemAddress = extern "C" fn();

/// NetworkPartition that processes the ports in sequence and performs
/// registered actions on them.
/// loop
///   sample_each_sampling_port_destination
///     match data type / port name
///       perform registered actions for match
/// TODO must be able to iterate over all destinations
#[derive(Debug)]
pub struct NetworkPartition<
    const PORT_MTU: MessageSize,
    const TABLE_SIZE: usize,
    const INTERFACES: usize,
    H: ApexSamplingPortP4 + 'static,
> {
    config: Config<TABLE_SIZE, TABLE_SIZE, INTERFACES>,
    router: &'static OnceCell<Router<TABLE_SIZE>>,
    // TODO make into struct
    source_ports:
        &'static OnceCell<LinearMap<ChannelId, SamplingPortSource<PORT_MTU, H>, TABLE_SIZE>>,
    // TODO make into struct
    destination_ports:
        &'static OnceCell<LinearMap<ChannelId, SamplingPortDestination<PORT_MTU, H>, TABLE_SIZE>>,
    entry_point: SystemAddress,
}

impl<const ECHO_SIZE: MessageSize, const TABLE_SIZE: usize, const INTERFACES: usize, H>
    NetworkPartition<ECHO_SIZE, TABLE_SIZE, INTERFACES, H>
where
    H: ApexSamplingPortP4,
{
    /// Create a new instance of the network partition
    pub fn new(
        config: Config<TABLE_SIZE, TABLE_SIZE, INTERFACES>,
        router: &'static OnceCell<Router<TABLE_SIZE>>,
        source_ports: &'static OnceCell<
            LinearMap<ChannelId, SamplingPortSource<ECHO_SIZE, H>, TABLE_SIZE>,
        >,
        destination_ports: &'static OnceCell<
            LinearMap<ChannelId, SamplingPortDestination<ECHO_SIZE, H>, TABLE_SIZE>,
        >,
        entry_point: SystemAddress,
    ) -> Self {
        NetworkPartition::<ECHO_SIZE, TABLE_SIZE, INTERFACES, H> {
            config,
            router,
            source_ports,
            destination_ports,
            entry_point,
        }
    }
}

// TODO create all ports and processes from config
impl<const MSG_SIZE: MessageSize, const TABLE_SIZE: usize, const INTERFACES: usize, H> Partition<H>
    for NetworkPartition<MSG_SIZE, TABLE_SIZE, INTERFACES, H>
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4 + Debug,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        let mut router = Router::<TABLE_SIZE>::default();

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

        let mut destination_ports: LinearMap<
            ChannelId,
            SamplingPortDestination<MSG_SIZE, H>,
            TABLE_SIZE,
        > = LinearMap::default();

        if let Some(config) = echo_request_port_config {
            let name = Name::from_str("EchoRequest").unwrap();
            let port = ctx
                .create_sampling_port_destination::<MSG_SIZE>(name, config.validity)
                .unwrap();
            _ = destination_ports.insert(ChannelId::from(0), port).unwrap();
        }

        self.destination_ports.set(destination_ports).unwrap();

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

        let mut source_ports: LinearMap<ChannelId, SamplingPortSource<MSG_SIZE, H>, TABLE_SIZE> =
            LinearMap::default();

        if echo_reply_port_config.is_some() {
            let port = ctx
                .create_sampling_port_source::<MSG_SIZE>(Name::from_str("EchoReply").unwrap())
                .unwrap();

            _ = source_ports.insert(ChannelId::from(1), port).unwrap();
            // TODO Loopback table
            router
                .add_output_route(ChannelId::from(0), VirtualLinkId::from(0))
                .unwrap(); // TODO add virtual link with ID 0
            router
                .add_input_route(VirtualLinkId::from(0), ChannelId::from(1))
                .unwrap();
        }

        self.source_ports.set(source_ports).unwrap();
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

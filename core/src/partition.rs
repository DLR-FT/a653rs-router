use crate::config::*;
use apex_rs::prelude::*;
use core::fmt::Debug;
use core::str::FromStr;
use core::time::Duration;

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
    const MTU: MessageSize,
    const PORTS: usize,
    const INTERFACES: usize,
    const MAX_QUEUE_LEN: usize,
    const LINKS: usize,
> where
    [(); MTU as usize]:,
{
    config: Config<PORTS, LINKS, INTERFACES>,
    //links: &'static OnceCell<Vec<VirtualLink, LINKS>>,
    entry_point: SystemAddress,
}

impl<
        const MTU: MessageSize,
        const PORTS: usize,
        const INTERFACES: usize,
        const MAX_QUEUE_LEN: usize,
        const LINKS: usize,
    > NetworkPartition<MTU, PORTS, INTERFACES, MAX_QUEUE_LEN, LINKS>
where
    [(); MTU as usize]:,
{
    /// Create a new instance of the network partition
    pub fn new(
        config: Config<PORTS, LINKS, INTERFACES>,
        //links: &'static OnceCell<Vec<VirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>, LINKS>>,
        entry_point: SystemAddress,
    ) -> Self {
        NetworkPartition::<MTU, PORTS, INTERFACES, MAX_QUEUE_LEN, LINKS> {
            config,
            entry_point,
        }
    }
}

// TODO create all ports and processes from config
impl<
        const MTU: MessageSize,
        const PORTS: usize,
        const INTERFACES: usize,
        const MAX_QUEUE_LEN: usize,
        H,
        const LINKS: usize,
    > Partition<H> for NetworkPartition<MTU, PORTS, INTERFACES, MAX_QUEUE_LEN, LINKS>
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4 + Debug,
    [(); MTU as usize]:,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
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
            let _port = ctx
                .create_sampling_port_destination::<MTU>(name, config.validity)
                .unwrap();
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

        if echo_reply_port_config.is_some() {
            _ = ctx
                .create_sampling_port_source::<MTU>(Name::from_str("EchoReply").unwrap())
                .unwrap();
        }

        // TODO init links

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

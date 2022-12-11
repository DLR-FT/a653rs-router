use apex_rs::prelude::*;
use core::fmt::Debug;
use core::str::FromStr;
use core::time::Duration;

type SystemAddress = extern "C" fn();

/// Network partition.
#[derive(Debug)]
pub struct NetworkPartition {
    stack_size: StackSize,
    entry_point: SystemAddress,
}

impl NetworkPartition {
    pub fn new(stack_size: StackSize, entry_point: SystemAddress) -> Self {
        Self {
            stack_size,
            entry_point,
        }
    }
}

// TODO generate based on config
// PORT_NAME0 = OnceCell
// PORT_NAME1 = OnceCell
// ...
// INTERFACE_NAME0 = OnceCell

// TODO create all ports and processes from config
impl<H> Partition<H> for NetworkPartition
where
    H: ApexSamplingPortP4 + ApexProcessP4 + ApexPartitionP4 + Debug,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        // TODO init ports
        // TODO init interfaces
        // TODO generate based on config
        // PORT_{cfg.name.upper()}.set(ctx.create_sampling_port_destination::<{cfg.msg_size], H>({cfg.name}, {config.validity}).unwrap());
        // INTERFACE_{cfg.name.upper()}.set

        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::ZERO),
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point,
            stack_size: self.stack_size,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("network_partition").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap()
    }

    fn warm_start(&self, ctx: &mut StartContext<H>) {
        self.cold_start(ctx)
    }
}

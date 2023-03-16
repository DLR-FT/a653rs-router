use crate::{
    network::PayloadSize,
    prelude::{DataRate, NetworkInterfaceId, VirtualLinkId},
};

use apex_rs::bindings::MessageRange;
use core::fmt::Display;
use core::time::Duration;
use heapless::{String, Vec};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize};

#[cfg(feature = "std")]
use bytesize::ByteSize;

const MAX_NAME_LEN: usize = 20;

/// The name of a channel.
type ChannelName = String<MAX_NAME_LEN>;

/// Configuration of the network partition
///
/// # Examples
/// ```rust
/// use core::time::Duration;
/// use network_partition::prelude::*;
/// use heapless::{String, Vec};
///
/// let config = Config::<10, 10, 10, 2> {
///     stack_size: StackSizeConfig {
///       aperiodic_process: 100000,
///     },
///     virtual_links: Vec::from_slice(&[
///         VirtualLinkConfig::<10, 10> {
///             id: VirtualLinkId::from(0),
///             msg_size: 1000,
///             interfaces: Vec::from_slice(&[InterfaceName::from("veth0"), InterfaceName::from("veth1")]).unwrap(),
///             fifo_depth: None,
///             ports: Vec::from_slice(&[
///                 Port::SamplingPortDestination(SamplingPortDestinationConfig {
///                     channel: String::from("EchoRequest"),
///                     validity: Duration::from_secs(1),
///                 }),
///                 Port::SamplingPortSource(SamplingPortSourceConfig {
///                     channel: String::from("EchoReply"),
///                 }),
///             ]).unwrap(),
///         },
///         VirtualLinkConfig {
///             id: VirtualLinkId::from(1),
///             msg_size: 1000,
///             fifo_depth: None,
///             ports:  Vec::default(),
///             interfaces: Vec::default(),
///         }
///     ]).unwrap(),
///     interfaces: Vec::from_slice(&[
///         InterfaceConfig::Udp(UdpInterfaceConfig {
///             id: NetworkInterfaceId::from(1),
///             name: InterfaceName::from("8081"),
///             rate: DataRate::b(10000000),
///             mtu: 1000,
///             destination: String::from("127.0.0.1:8000"),
///         }),
///     ]).unwrap(),
///     schedule: ScheduleConfig::DeadlineRr(DeadlineRrScheduleConfig::<2> { slots: Vec::from_slice(&[
///         DeadlineRrSlot { vl: VirtualLinkId::from(0), period: Duration::from_millis(100)},
///         DeadlineRrSlot { vl: VirtualLinkId::from(1), period: Duration::from_millis(50)},
///     ]).unwrap()}),
/// };
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Config<
    const PORTS: usize,
    const IFS: usize,
    const VLS: usize,
    const SCHEDULE_SLOTS: usize,
> {
    /// The amount of memory to reserve on the stack for the processes of the partition.
    pub stack_size: StackSizeConfig,

    /// The virtual links the partition is attached to.
    pub virtual_links: Vec<VirtualLinkConfig<PORTS, IFS>, VLS>,

    /// The interfaces that will be attached to the partition.
    #[cfg_attr(feature = "serde", serde(default = "default_interfaces"))]
    pub interfaces: Vec<InterfaceConfig, IFS>,

    /// Configuration for the scheduler.
    pub schedule: ScheduleConfig<SCHEDULE_SLOTS>,
}

fn default_interfaces<const IFS: usize>() -> Vec<InterfaceConfig, IFS> {
    Vec::new()
}

/// Configures the amount of stack memory to reserve for the prcesses of the partition.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct StackSizeConfig {
    /// The size of the memory to reserve on the stack for the aperiodic process.
    #[cfg_attr(
        all(feature = "serde", feature = "std"),
        serde(deserialize_with = "de_size_str_u32")
    )]
    pub aperiodic_process: u32,
}

// TODO add enum for VirtualQueuingLink and VirtualSamplingLink

/// Configuration for a virtual link.
///
/// Virtual links are used to connect multiple network partitions.
/// Each virtual link can have exactly one source and one or more destinations.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct VirtualLinkConfig<const PORTS: usize, const IFS: usize> {
    /// The unique ID of the virtual link
    pub id: VirtualLinkId,

    /// The maximum size of a message that will be transmited using this virtual link.
    #[cfg_attr(
        all(feature = "serde", feature = "std"),
        serde(deserialize_with = "de_size_str_u32")
    )]
    pub msg_size: PayloadSize,

    /// The depth of the attached queueing channels.
    /// This intentionally enforces that all queues have the same size.
    /// Having larger receiver queues than sender queues would waste resources.
    /// Having larger sender queues than receiver queues would not be safe (e.g. dropped messages).
    /// APEX queueing channels only have one queue for sender / receiver. This is a translation of this concept to a virtual link.
    pub fifo_depth: Option<MessageRange>,

    /// The ports the virtual link should create to connect to channels.
    pub ports: Vec<Port, PORTS>,

    /// The interfaces that are attached
    #[cfg_attr(feature = "serde", serde(default = "default_interface_names"))]
    pub interfaces: Vec<InterfaceName, IFS>,
}

fn default_interface_names<const IFS: usize>() -> Vec<InterfaceName, IFS> {
    Vec::default()
}

/// The name of an interface. The name is platform-dependent.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct InterfaceName(pub String<MAX_NAME_LEN>);

impl From<&str> for InterfaceName {
    #[inline]
    fn from(s: &str) -> Self {
        Self(String::from(s))
    }
}

impl Display for InterfaceName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Interface configuration.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum InterfaceConfig {
    /// An interface implementation that is attached to a UDP socket on linux.
    Udp(UdpInterfaceConfig),
    /// An interface that is attached to a UART PL on a Zynq 7000.
    Uart(UartInterfaceConfig),
}

/// UART interfacew configuration.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct UartInterfaceConfig {
    /// Id of the interface
    pub id: NetworkInterfaceId,

    /// Name of the interface. Used in virtual link config.
    pub name: InterfaceName,

    /// The maximum size of a message that will be transmited using this virtual link.
    #[cfg_attr(
        all(feature = "serde", feature = "std"),
        serde(deserialize_with = "de_size_str_u32")
    )]
    pub mtu: PayloadSize,
}

// TODO move Linux networking into network-partitin crate and hide behind std feature, likewise for xng networking

/// Configuration for an UDP "interface".
///
/// Interfaces are used to connect multiple hypervisors and transmit all virtual links.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct UdpInterfaceConfig {
    /// Id of the interface
    pub id: NetworkInterfaceId,

    /// The unique ID of the interface.
    pub name: InterfaceName,

    /// The maximum rate the interface can transmit at.
    pub rate: DataRate,

    /// The maximum size of a message that will be transmited using this virtual link.
    #[cfg_attr(
        all(feature = "serde", feature = "std"),
        serde(deserialize_with = "de_size_str_u32")
    )]
    pub mtu: PayloadSize,

    /// UDP destination peer
    /// TODO remove
    pub destination: String<MAX_NAME_LEN>,
}

/// A port of a communication channel with the hypervisor.
///
/// Ports destinations and sources are created by partitions to attach to a port.
/// Ports provide acces to communication channels between partitions.
/// There are cirrently two types of ports implemented, for the sending and receiving ends of sampling ports.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum Port {
    /// Source port of a sampling channel.
    SamplingPortSource(SamplingPortSourceConfig),

    /// Destination port of a sampling channel.
    SamplingPortDestination(SamplingPortDestinationConfig),

    /// Source port of a queuing channel.
    QueuingPortSender(QueuingPortConfig),

    /// Destination port of a queuing channel.
    QueuingPortReceiver(QueuingPortConfig),
}

/// Parameters of a port that is attached to a queuing channel, either the receiver or the sender.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct QueuingPortConfig {
    /// The name of the channel this sender is attached to.
    pub channel: ChannelName,
}

/// Configuration for a sampling port destination.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct SamplingPortDestinationConfig {
    /// The name of the channel the port should be attached to.
    pub channel: ChannelName,

    /// The amount of time a message that is stored inside the channel is considered valid.
    ///
    /// The hypervisor will tell us, if the message is still valid, when we read it.
    #[cfg_attr(feature = "std", serde(with = "humantime_serde"))]
    pub validity: Duration,
}

/// Configuration for a sampling port source.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct SamplingPortSourceConfig {
    /// The name of the channel the port should be attached to.
    pub channel: ChannelName,
}

impl Port {
    /// Tries to destructure the config of the destination port.
    pub fn sampling_port_destination(&self) -> Option<SamplingPortDestinationConfig> {
        if let Self::SamplingPortDestination(q) = self {
            return Some(q.clone());
        }
        None
    }

    /// Tries to destructure the config of the source port.
    pub fn sampling_port_source(&self) -> Option<SamplingPortSourceConfig> {
        if let Self::SamplingPortSource(q) = self {
            return Some(q.clone());
        }
        None
    }

    /// Tries to destructure the config of the sender port.
    pub fn queuing_port_sender(&self) -> Option<QueuingPortConfig> {
        if let Self::QueuingPortSender(q) = self {
            return Some(q.clone());
        }
        None
    }

    /// Tries to destructure the config of the receiver port.
    pub fn queuing_port_receiver(&self) -> Option<QueuingPortConfig> {
        if let Self::QueuingPortReceiver(q) = self {
            return Some(q.clone());
        }
        None
    }
}

const MAX_BYTE_SIZE: usize = 20;

#[cfg(all(feature = "std", feature = "serde"))]
fn de_size_str<'de, D>(de: D) -> Result<ByteSize, D::Error>
where
    D: Deserializer<'de>,
{
    String::<MAX_BYTE_SIZE>::deserialize(de)?
        .parse::<ByteSize>()
        .map_err(serde::de::Error::custom)
}

#[cfg(all(feature = "std", feature = "serde"))]
fn de_size_str_u32<'de, D>(de: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    de_size_str(de).map(|r| r.as_u64() as u32)
}

/// Scheduler confgiguration.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum ScheduleConfig<const SCHEDULE_SLOTS: usize> {
    /// This configuration requires the deadline-based round-robin scheduler.
    DeadlineRr(DeadlineRrScheduleConfig<SCHEDULE_SLOTS>),
}

impl<const SLOTS: usize> ScheduleConfig<SLOTS> {
    /// Gets the deadline RR scheduler config.
    pub fn deadline_rr(self) -> Option<DeadlineRrScheduleConfig<SLOTS>> {
        match self {
            Self::DeadlineRr(cfg) => Some(cfg),
        }
    }
}

/// Configuration for the deadline-based round-robin scheduler.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct DeadlineRrScheduleConfig<const SCHEDULE_SLOTS: usize> {
    /// Shedule slots.
    pub slots: Vec<DeadlineRrSlot, SCHEDULE_SLOTS>,
}

/// A slot inside the round-robin scheduler.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct DeadlineRrSlot {
    /// Virtual link to schedule in this slot.
    pub vl: VirtualLinkId,

    /// Periodic after which to schedule this slot again after the last time it has been scheduled.
    #[cfg_attr(all(feature = "std"), serde(with = "humantime_serde"))]
    pub period: Duration,
}

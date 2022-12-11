use bytesize::ByteSize;
use core::time::Duration;
use heapless::String;
use heapless::Vec;
use serde::{Deserialize, Deserializer, Serialize};

const MAX_CHANNEL_NAME: usize = 32;

/// The ID of a virtual link.
///
/// TODO Actual size might depend on network layer.
/// Might be size of VLAN tag or virtual link id or something else.
type VirtualLinkId = u16;

/// The name of a channel.
type ChannelName = String<MAX_CHANNEL_NAME>;

/// Configuration of the network partition
///
/// # Examples
/// ```rust
/// use bytesize::ByteSize;
/// use std::time::Duration;
/// use network_partition::prelude::*;
///
/// let config = Config::<10, 10, 10> {
///     stack_size: StackSizeConfig {
///       periodic_process: ByteSize::kb(100),
///     },
///     ports: heapless::Vec::from_slice(&[
///         Port::SamplingPortDestination(SamplingPortDestinationConfig {
///             channel: heapless::String::from("EchoRequest"),
///             msg_size: ByteSize::kb(2),
///             validity: Duration::from_secs(1),
///             virtual_link: 0,
///         }),
///         Port::SamplingPortSource(SamplingPortSourceConfig {
///             channel: heapless::String::from("EchoReply"),
///             msg_size: ByteSize::kb(2),
///             virtual_link: 1,
///         }),
///     ]).unwrap(),
///     virtual_links: heapless::Vec::from_slice(&[
///         VirtualLinkConfig {
///             id: 0,
///             rate: DataRate::b(1000),
///             msg_size: ByteSize::kb(1),
///             interfaces: heapless::Vec::from_slice(&[InterfaceName::from("veth0"), InterfaceName::from("veth1")]).unwrap()
///         },
///         VirtualLinkConfig {
///             id: 1,
///             rate: DataRate::b(1000),
///             msg_size: ByteSize::kb(1),
///             interfaces: heapless::Vec::from_slice(&[]).unwrap(),
///         }
///     ]).unwrap(),
///     interfaces: heapless::Vec::from_slice(&[
///        InterfaceConfig {
///            name: InterfaceName::from("veth0"),
///            rate: DataRate::b(10000000),
///            mtu: ByteSize::kb(1),
///        },
///     ]).unwrap()
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config<const PORTS: usize, const LINKS: usize, const INTERFACES: usize> {
    /// The amount of memory to reserve on the stack for the processes of the partition.
    pub stack_size: StackSizeConfig,

    /// The ports the partition should create to connect to channels.
    pub ports: Vec<Port, PORTS>,

    /// The virtual links the partition is attached to.
    pub virtual_links: Vec<VirtualLinkConfig<INTERFACES>, LINKS>,

    /// The interfaces that will be attached to the partition.
    pub interfaces: Vec<InterfaceConfig, INTERFACES>,
}

/// Configures the amount of stack memory to reserve for the prcesses of the partition.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StackSizeConfig {
    /// The size of the memory to reserve on the stack for the periodic process.
    #[serde(deserialize_with = "de_size_str")]
    pub periodic_process: ByteSize,
}

/// A data-rate in bit/s.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct DataRate(u64);

impl DataRate {
    /// Constructs a data rate from a `u64` in bits/s.
    pub const fn b(bits: u64) -> Self {
        Self(bits)
    }

    /// Gets the bits/s as a `u64`.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Configuration for a virtual link.
///
/// Virtual links are used to connect multiple network partitions.
/// Each virtual link can have exactly one source and one or more destinations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VirtualLinkConfig<const INTERFACES: usize> {
    /// The unique ID of the virtual link
    pub id: VirtualLinkId,

    /// The maximum rate the link may transmit at.
    pub rate: DataRate,

    /// The maximum size of a message that will be transmited using this virtual link.
    #[serde(deserialize_with = "de_size_str")]
    pub msg_size: ByteSize,

    /// The interfaces that are attached
    pub interfaces: Vec<InterfaceName, INTERFACES>,
}

const MAX_INTERFACE_NAME: usize = 10;

/// The name of an interface. The name is platform-dependent.
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct InterfaceName(String<MAX_INTERFACE_NAME>);

impl From<&str> for InterfaceName {
    fn from(val: &str) -> Self {
        Self(String::from(val))
    }
}

/// Configuration for an interface.
///
/// Interfaces are used to connect multiple hypervisors and transmit all virtual links.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceConfig {
    /// The unique ID of the virtual link
    pub name: InterfaceName,

    /// The maximum rate the interface can transmit at.
    pub rate: DataRate,

    /// The maximum size of a message that will be transmited using this virtual link.
    #[serde(deserialize_with = "de_size_str")]
    pub mtu: ByteSize,
}

/// A port of a communication channel with the hypervisor.
///
/// Ports destinations and sources are created by partitions to attach to a port.
/// Ports provide acces to communication channels between partitions.
/// There are cirrently two types of ports implemented, for the sending and receiving ends of sampling ports.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Port {
    /// Source port of a sampling channel.
    SamplingPortSource(SamplingPortSourceConfig),

    /// Destination port of a sampling channel.
    SamplingPortDestination(SamplingPortDestinationConfig),
}

/// Configuration for a sampling port destination.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SamplingPortDestinationConfig {
    /// The name of the channel the port should be attached to.
    pub channel: ChannelName,

    /// The maximum size of a single message that can be transmitted using the port.
    #[serde(deserialize_with = "de_size_str")]
    pub msg_size: ByteSize,

    /// The amount of time a message that is stored inside the channel is considered valid.
    ///
    /// The hypervisor will tell us, if the message is still valid, when we read it.
    #[serde(with = "humantime_serde")]
    pub validity: Duration,

    /// The virtual links to forward messages from this port to.
    ///
    /// A single port may send messages only to a single virtual link. This is neccessary to identify which sender created the data.
    pub virtual_link: VirtualLinkId,
}

/// Configuration for a sampling port source.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SamplingPortSourceConfig {
    /// The name of the channel the port should be attached to.
    pub channel: ChannelName,

    /// The maximum size of a single message that can be transmitted using the port.
    #[serde(deserialize_with = "de_size_str")]
    pub msg_size: ByteSize,

    /// The virtual link from which to forward messages to this port.
    ///
    /// A single port may receive messages only from a single virtual link. This is neccessary to identify which sender created the data.
    pub virtual_link: VirtualLinkId,
}

fn de_size_str<'de, D>(de: D) -> Result<ByteSize, D::Error>
where
    D: Deserializer<'de>,
{
    String::<MAX_CHANNEL_NAME>::deserialize(de)?
        .parse::<ByteSize>()
        .map_err(serde::de::Error::custom)
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
}

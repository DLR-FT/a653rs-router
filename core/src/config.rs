use crate::{
    network::PayloadSize,
    prelude::{DataRate, NetworkInterfaceId, VirtualLinkId},
};

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
/// let config = Config::<10, 10, 10> {
///     stack_size: StackSizeConfig {
///       aperiodic_process: 100000,
///     },
///     virtual_links: Vec::from_slice(&[
///         VirtualLinkConfig::<10, 10> {
///             id: VirtualLinkId::from(0),
///             rate: Duration::from_millis(1000),
///             msg_size: 1000,
///             interfaces: Vec::from_slice(&[InterfaceName::from("veth0"), InterfaceName::from("veth1")]).unwrap(),
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
///             rate: Duration::from_millis(1000),
///             ports:  Vec::default(),
///             interfaces: Vec::default(),
///         }
///     ]).unwrap(),
///     interfaces: Vec::from_slice(&[
///        InterfaceConfig {
///            id: NetworkInterfaceId::from(1),
///            name: InterfaceName::from("veth0"),
///            rate: DataRate::b(10000000),
///            mtu: 1000,
///            destination: String::from("127.0.0.1:8000"),
///        },
///     ]).unwrap()
/// };
/// ```
#[cfg(feature = "std")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Config<const PORTS: usize, const IFS: usize, const VLS: usize> {
    /// The amount of memory to reserve on the stack for the processes of the partition.
    pub stack_size: StackSizeConfig,

    /// The virtual links the partition is attached to.
    pub virtual_links: Vec<VirtualLinkConfig<PORTS, IFS>, VLS>,

    /// The interfaces that will be attached to the partition.
    #[cfg_attr(feature = "serde", serde(default = "default_interfaces"))]
    pub interfaces: Vec<InterfaceConfig, IFS>,
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

/// Configuration for a virtual link.
///
/// Virtual links are used to connect multiple network partitions.
/// Each virtual link can have exactly one source and one or more destinations.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct VirtualLinkConfig<const PORTS: usize, const IFS: usize> {
    /// The unique ID of the virtual link
    pub id: VirtualLinkId,

    /// The maximum rate the link may transmit at.
    #[cfg_attr(all(feature = "std"), serde(with = "humantime_serde"))]
    pub rate: Duration,

    /// The maximum size of a message that will be transmited using this virtual link.
    #[cfg_attr(
        all(feature = "serde", feature = "std"),
        serde(deserialize_with = "de_size_str_u32")
    )]
    pub msg_size: PayloadSize,

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
#[derive(Debug, Clone)]
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

/// Configuration for an interface.
///
/// Interfaces are used to connect multiple hypervisors and transmit all virtual links.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct InterfaceConfig {
    /// Id of the interface
    pub id: NetworkInterfaceId,

    /// The unique ID of the virtual link
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
    de_size_str(de).and_then(|r| Ok(r.as_u64() as u32))
}

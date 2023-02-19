use crate::prelude::{DataRate, NetworkInterfaceId, VirtualLinkId};
use bytesize::ByteSize;
use core::fmt::Display;
use core::time::Duration;
use heapless::{String, Vec};
use serde::{Deserialize, Deserializer, Serialize};

const MAX_NAME_LEN: usize = 20;

/// The name of a channel.
type ChannelName = String<MAX_NAME_LEN>;

#[derive(Serialize, Deserialize)]
#[serde(remote = "DataRate")]
struct DataRateDef(u64);

#[derive(Serialize, Deserialize)]
#[serde(remote = "VirtualLinkId")]
struct VirtualLinkIdDef(u32);

#[derive(Serialize, Deserialize)]
#[serde(remote = "NetworkInterfaceId")]
struct NetworkInterfaceIdDef(u32);

/// Configuration of the network partition
///
/// # Examples
/// ```rust
/// use bytesize::ByteSize;
/// use std::time::Duration;
/// use network_partition_config::config::*;
/// use network_partition::prelude::*;
///
/// let config = Config {
///     stack_size: StackSizeConfig {
///       periodic_process: ByteSize::kb(100),
///     },
///     virtual_links: Vec::from_slice(&[
///         VirtualLinkConfig {
///             id: VirtualLinkId::from(0),
///             rate: Duration::from_millis(1000),
///             msg_size: ByteSize::kb(1),
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
///             msg_size: ByteSize::kb(1),
///             rate: Duration::from_millis(1000),
///             ports:  Vec::default(),
///         }
///     ]).unwrap(),
///     interfaces: Vec::from_slice!(&[
///        InterfaceConfig {
///            id: NetworkInterfaceId::from(1),
///            name: InterfaceName::from("veth0"),
///            rate: DataRate::b(10000000),
///            mtu: ByteSize::kb(1),
///            destination: String::from("127.0.0.1:8000"),
///        },
///     ]).unwrap()
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config<const VLS: usize, const PORTS: usize, const IFS: usize> {
    /// The amount of memory to reserve on the stack for the processes of the partition.
    pub stack_size: StackSizeConfig,

    /// The virtual links the partition is attached to.
    pub virtual_links: Vec<VirtualLinkConfig<PORTS, IFS>, VLS>,

    /// The interfaces that will be attached to the partition.
    #[serde(default = "default_interfaces")]
    pub interfaces: Vec<InterfaceConfig, IFS>,
}

fn default_interfaces<const IFS: usize>() -> Vec<InterfaceConfig, IFS> {
    Vec::new()
}

/// Configures the amount of stack memory to reserve for the prcesses of the partition.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StackSizeConfig {
    /// The size of the memory to reserve on the stack for the periodic process.
    #[serde(deserialize_with = "de_size_str")]
    pub periodic_process: ByteSize,
}

/// Configuration for a virtual link.
///
/// Virtual links are used to connect multiple network partitions.
/// Each virtual link can have exactly one source and one or more destinations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VirtualLinkConfig<const PORTS: usize, const IFS: usize> {
    /// The unique ID of the virtual link
    #[serde(with = "VirtualLinkIdDef")]
    pub id: VirtualLinkId,

    /// The maximum rate the link may transmit at.
    #[serde(with = "humantime_serde")]
    pub rate: Duration,

    /// The maximum size of a message that will be transmited using this virtual link.
    #[serde(deserialize_with = "de_size_str")]
    pub msg_size: ByteSize,

    /// The ports the virtual link should create to connect to channels.
    pub ports: Vec<Port, PORTS>,

    /// The interfaces that are attached
    #[serde(default = "default_interface_names")]
    pub interfaces: Vec<InterfaceName, IFS>,
}

fn default_interface_names<const IFS: usize>() -> Vec<InterfaceName, IFS> {
    Vec::default()
}

/// The name of an interface. The name is platform-dependent.
#[derive(Debug, Serialize, Clone, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceConfig {
    /// Id of the interface
    #[serde(with = "NetworkInterfaceIdDef")]
    pub id: NetworkInterfaceId,

    /// The unique ID of the virtual link
    pub name: InterfaceName,

    /// The maximum rate the interface can transmit at.
    #[serde(with = "DataRateDef")]
    pub rate: DataRate,

    /// The maximum size of a message that will be transmited using this virtual link.
    #[serde(deserialize_with = "de_size_str")]
    pub mtu: ByteSize,

    /// UDP destination peer
    /// TODO remove
    pub destination: String<MAX_NAME_LEN>,
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

    /// The amount of time a message that is stored inside the channel is considered valid.
    ///
    /// The hypervisor will tell us, if the message is still valid, when we read it.
    #[serde(with = "humantime_serde")]
    pub validity: Duration,
}

/// Configuration for a sampling port source.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SamplingPortSourceConfig {
    /// The name of the channel the port should be attached to.
    pub channel: ChannelName,
}

const MAX_BYTE_SIZE: usize = 20;

fn de_size_str<'de, D>(de: D) -> Result<ByteSize, D::Error>
where
    D: Deserializer<'de>,
{
    String::<MAX_BYTE_SIZE>::deserialize(de)?
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

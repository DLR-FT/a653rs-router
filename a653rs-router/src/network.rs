use crate::{
    router::{PortError, RouterInput, RouterOutput},
    types::{DataRate, VirtualLinkId},
};

use core::{
    fmt::{Display, Formatter},
    marker::PhantomData,
};
use heapless::String;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Size of a frame payload.
pub type PayloadSize = usize;

/// Network interface ID.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NetworkInterfaceId(pub u32);

#[allow(clippy::from_over_into)]
impl Into<usize> for NetworkInterfaceId {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl From<usize> for NetworkInterfaceId {
    fn from(val: usize) -> Self {
        Self(val as u32)
    }
}

/// A network interface.
#[derive(Debug, Clone)]
pub struct NetworkInterface<const MTU: PayloadSize, H: PlatformNetworkInterface> {
    _h: PhantomData<H>,
    id: NetworkInterfaceId,
}

impl<const MTU: PayloadSize, H: PlatformNetworkInterface> NetworkInterface<MTU, H> {
    /// ID of this interface.
    pub fn id(&self) -> NetworkInterfaceId {
        self.id
    }

    /// Sends data to the interface.
    pub fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<usize, InterfaceError> {
        if buf.len() > MTU {
            return Err(InterfaceError::InsufficientBuffer);
        }

        router_trace!("Sending to interface");
        H::platform_interface_send_unchecked(self.id, *vl, buf)
    }

    /// Receives data from the interface.
    pub fn receive<'a>(
        &self,
        buf: &'a mut [u8],
    ) -> Result<(VirtualLinkId, &'a [u8]), InterfaceError> {
        if buf.len() < MTU {
            return Err(InterfaceError::InsufficientBuffer);
        }

        H::platform_interface_receive_unchecked(self.id, buf)
    }
}

/// Platform-specific network interface type.
pub trait PlatformNetworkInterface {
    /// The configuration for this interface. May be any struct.
    type Configuration;

    /// Send something to the network and report how long it took.
    fn platform_interface_send_unchecked(
        id: NetworkInterfaceId,
        vl: VirtualLinkId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError>;

    /// Receive something from the network and report the virtual link id and
    /// the payload.
    fn platform_interface_receive_unchecked(
        id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<(VirtualLinkId, &'_ [u8]), InterfaceError>;
}

/// Creates a network interface id.
pub trait CreateNetworkInterfaceId<H: PlatformNetworkInterface> {
    /// Creates a network interface id.
    fn create_network_interface_id(
        cfg: H::Configuration,
    ) -> Result<NetworkInterfaceId, InterfaceError>;
}

/// Creates a nertwork interface with an MTU.
pub trait CreateNetworkInterface<H: PlatformNetworkInterface> {
    /// Creates a network interface id.
    fn create_network_interface<const MTU: PayloadSize>(
        cfg: H::Configuration,
    ) -> Result<NetworkInterface<MTU, H>, InterfaceError>;
}

impl<H: PlatformNetworkInterface, T> CreateNetworkInterface<H> for T
where
    T: CreateNetworkInterfaceId<H>,
{
    fn create_network_interface<const MTU: PayloadSize>(
        cfg: H::Configuration,
    ) -> Result<NetworkInterface<MTU, H>, InterfaceError> {
        Ok(NetworkInterface {
            _h: PhantomData,
            id: T::create_network_interface_id(cfg)?,
        })
    }
}

const MAX_SOCKET_NAME: usize = 50;

/// Configuration for an interface.
#[derive(Debug, Clone)]
pub struct InterfaceConfig {
    /// UDP source address the socket is bound to.
    pub source: String<MAX_SOCKET_NAME>,

    /// The maximum rate the interface can transmit at.
    pub rate: DataRate,

    /// The maximum size of a message that will be transmited using this virtual
    /// link.
    pub mtu: PayloadSize,

    /// UDP destination peer
    pub destination: String<MAX_SOCKET_NAME>,
}

impl InterfaceConfig {
    /// Creates a new configuration.
    pub fn new(source: &str, destination: &str, rate: DataRate, mtu: PayloadSize) -> Self {
        Self {
            source: String::from(source),
            destination: String::from(destination),
            rate,
            mtu,
        }
    }
}

impl<const M: PayloadSize, H: PlatformNetworkInterface> RouterInput for NetworkInterface<M, H> {
    fn receive<'a>(
        &self,
        _vl: &VirtualLinkId,
        buf: &'a mut [u8],
    ) -> Result<(VirtualLinkId, &'a [u8]), PortError> {
        NetworkInterface::receive(self, buf).map_err(|_e| PortError::Receive)
    }
}

impl<const M: PayloadSize, H: PlatformNetworkInterface> RouterOutput for NetworkInterface<M, H> {
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), PortError> {
        NetworkInterface::send(self, vl, buf)
            .map(|_l| ())
            .map_err(|_e| PortError::Send)
    }
}

/// Inteface error type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterfaceError {
    /// Insufficient buffer space
    InsufficientBuffer,
    /// No data available
    NoData,
    /// Invalid data received from interface
    InvalidData,
    /// Interface not found
    NotFound,
    /// Sending failed
    SendFailed,
}

impl Display for InterfaceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoData => write!(f, "No data available"),
            Self::InsufficientBuffer => write!(f, "Insufficient buffer space"),
            Self::InvalidData => write!(f, "Invalid data"),
            Self::NotFound => write!(f, "Interface not found"),
            Self::SendFailed => write!(f, "Send failed"),
        }
    }
}

use crate::{
    ports::PortError,
    router::{RouterInput, RouterOutput},
    types::DataRate,
};

use core::{
    fmt::{Display, Formatter},
    marker::PhantomData,
    str::FromStr,
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
pub struct NetworkInterface<P: PlatformNetworkInterface> {
    _p: PhantomData<P>,
    id: NetworkInterfaceId,
    mtu: PayloadSize,
}

impl<H: PlatformNetworkInterface> NetworkInterface<H> {
    /// Sends data to the interface.
    pub fn send(&self, buf: &[u8]) -> Result<usize, InterfaceError> {
        if buf.len() > self.mtu {
            return Err(InterfaceError::InsufficientBuffer);
        }

        router_trace!("Sending to interface");
        H::platform_interface_send_unchecked(self.id, buf)
    }

    /// Receives data from the interface.
    pub fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], InterfaceError> {
        if buf.len() < self.mtu {
            return Err(InterfaceError::InsufficientBuffer);
        }

        H::platform_interface_receive_unchecked(self.id, buf)
    }
}

/// Platform-specific network interface type.
pub trait PlatformNetworkInterface {
    /// Send something to the network and report how long it took.
    fn platform_interface_send_unchecked(
        id: NetworkInterfaceId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError>;

    /// Receive something from the network and report the virtual link id and
    /// the payload.
    fn platform_interface_receive_unchecked(
        id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<&'_ [u8], InterfaceError>;
}

/// Creates a network interface id.
pub trait CreateNetworkInterfaceId<H: PlatformNetworkInterface> {
    /// Creates a network interface id.
    fn create_network_interface_id(
        cfg: &InterfaceConfig,
    ) -> Result<NetworkInterfaceId, InterfaceError>;
}

/// Creates a nertwork interface with an MTU.
pub trait CreateNetworkInterface<H: PlatformNetworkInterface> {
    /// Creates a network interface id.
    fn create_network_interface(
        cfg: &InterfaceConfig,
    ) -> Result<NetworkInterface<H>, InterfaceError>;
}

impl<H: PlatformNetworkInterface, T> CreateNetworkInterface<H> for T
where
    T: CreateNetworkInterfaceId<H>,
{
    fn create_network_interface(
        cfg: &InterfaceConfig,
    ) -> Result<NetworkInterface<H>, InterfaceError> {
        Ok(NetworkInterface {
            _p: PhantomData,
            id: T::create_network_interface_id(cfg)?,
            mtu: cfg.mtu,
        })
    }
}

const MAX_SOCKET_NAME: usize = 50;

/// Configuration for an interface.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
            source: String::from_str(source).unwrap(),
            destination: String::from_str(destination).unwrap(),
            rate,
            mtu,
        }
    }
}

impl<H: PlatformNetworkInterface> RouterInput for NetworkInterface<H> {
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], PortError> {
        NetworkInterface::receive(self, buf).map_err(|e| {
            router_debug!("Failed to receive from network interface: {:?}", e);
            PortError::Receive
        })
    }

    fn mtu(&self) -> PayloadSize {
        self.mtu
    }
}

impl<H: PlatformNetworkInterface> RouterOutput for NetworkInterface<H> {
    fn send(&self, buf: &[u8]) -> Result<(), PortError> {
        NetworkInterface::send(self, buf).map(|_| ()).map_err(|e| {
            router_debug!("Failed to send to network interface: {:?}", e);
            PortError::Send
        })
    }

    fn mtu(&self) -> PayloadSize {
        self.mtu
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

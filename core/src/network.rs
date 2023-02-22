use core::marker::PhantomData;

use crate::prelude::*;
use log::trace;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Size of a frame payload.
pub type PayloadSize = u32;

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
    pub fn id(&self) -> NetworkInterfaceId {
        self.id
    }

    /// Sends data to the interface.
    pub fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<usize, Error> {
        if buf.len() > MTU as usize {
            return Err(Error::InterfaceSendFail(InterfaceError::InsufficientBuffer));
        }

        trace!("Sending to interface");
        match H::platform_interface_send_unchecked(self.id, *vl, buf) {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(Error::InterfaceSendFail(e)),
        }
    }

    /// Receives data from the interface.
    pub fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<(VirtualLinkId, &'a [u8]), Error> {
        if buf.len() < MTU as usize {
            return Err(Error::InterfaceReceiveFail(
                InterfaceError::InsufficientBuffer,
            ));
        }

        match H::platform_interface_receive_unchecked(self.id, buf) {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::InterfaceReceiveFail(e)),
        }
    }
}

/// Platform-specific network interface type.
pub trait PlatformNetworkInterface {
    /// Send something to the network and report how long it took.
    fn platform_interface_send_unchecked(
        id: NetworkInterfaceId,
        vl: VirtualLinkId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError>;

    /// Receive something from the network and report the virtual link id and the payload.
    fn platform_interface_receive_unchecked(
        id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<(VirtualLinkId, &'_ [u8]), InterfaceError>;
}

/// Creates a network interface id.
pub trait CreateNetworkInterfaceId<H: PlatformNetworkInterface> {
    /// Creates a network interface id.
    fn create_network_interface_id(
        _name: &str, // TODO use network_partition_config::config::InterfaceName ?
        destination: &str,
        rate: DataRate,
    ) -> Result<NetworkInterfaceId, InterfaceError>;
}

/// Creates a nertwork interface with an MTU.
pub trait CreateNetworkInterface<H: PlatformNetworkInterface> {
    /// Creates a network interface id.
    fn create_network_interface<const MTU: PayloadSize>(
        _name: &str, // TODO use network_partition_config::config::InterfaceName ?
        destination: &str,
        rate: DataRate,
    ) -> Result<NetworkInterface<MTU, H>, Error>;
}

impl<H: PlatformNetworkInterface, T> CreateNetworkInterface<H> for T
where
    T: CreateNetworkInterfaceId<H>,
{
    fn create_network_interface<const MTU: PayloadSize>(
        name: &str, // TODO use network_partition_config::config::InterfaceName ?
        destination: &str,
        rate: DataRate,
    ) -> Result<NetworkInterface<MTU, H>, Error> {
        let id = match T::create_network_interface_id(name, destination, rate) {
            Ok(id) => id,
            Err(e) => return Err(Error::InterfaceCreationError(e)),
        };
        Ok(NetworkInterface {
            _h: PhantomData::default(),
            id,
        })
    }
}

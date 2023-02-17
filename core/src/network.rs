use core::marker::PhantomData;

use crate::{
    error::Error,
    prelude::{DataRate, InterfaceError},
    virtual_link::VirtualLinkId,
};
use heapless::Vec;
use log::trace;

/// Size of a frame payload.
pub type PayloadSize = u32;

/// A frame that is managed by the queue.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Frame<const PL_SIZE: PayloadSize>
where
    [u8; PL_SIZE as usize]:,
{
    vl_id: VirtualLinkId,
    pl: Vec<u8, { PL_SIZE as usize }>,
}

impl<const PL_SIZE: PayloadSize> Frame<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    /// The contents of a frame.
    pub fn into_inner(self) -> (VirtualLinkId, Vec<u8, { PL_SIZE as usize }>) {
        (self.vl_id, self.pl)
    }

    /// Create a frame from a slice.
    pub fn from_parts(vl_id: VirtualLinkId, slice: &[u8]) -> Result<Self, ()> {
        Ok(Self {
            vl_id,
            pl: Vec::from_slice(slice)?,
        })
    }

    pub fn as_parts(&self) -> (VirtualLinkId, &[u8]) {
        (self.vl_id, self.pl.as_slice())
    }
}

/// A queue for storing frames that are waiting to be transmitted.
pub(crate) trait FrameQueue<const PL_SIZE: PayloadSize>
where
    [(); PL_SIZE as usize]:,
{
    /// Saves a frame to the queue to be written to the network later.
    /// If the underlying queue has no more free space, the oldest frame is dropped from the front
    /// and the new frame is inserted at the back.
    fn enqueue_frame(&mut self, frame: Frame<PL_SIZE>) -> Result<bool, Error>;

    /// Retrieves a frame from the queue to write it to the network.
    fn dequeue_frame(&mut self) -> Option<Frame<PL_SIZE>>;
}

impl<const PL_SIZE: PayloadSize, const QUEUE_CAPACITY: usize> FrameQueue<PL_SIZE>
    for heapless::spsc::Queue<Frame<PL_SIZE>, QUEUE_CAPACITY>
where
    [(); PL_SIZE as usize]:,
{
    fn enqueue_frame(&mut self, frame: Frame<PL_SIZE>) -> Result<bool, Error> {
        if self.len() < self.capacity() {
            match self.enqueue(frame) {
                Ok(_) => {
                    trace!("Enqueued frame without overflowing a queue.");
                    Ok(true)
                }
                Err(_) => Err(Error::EnqueueFailed),
            }
        } else {
            trace!("Dropping first frame from queue");
            _ = self.dequeue();
            match self.enqueue(frame) {
                Ok(_) => {
                    trace!("Enqueued frame while overflowing a queue.");
                    Ok(false)
                }
                Err(_) => Err(Error::EnqueueFailed),
            }
        }
    }

    fn dequeue_frame(&mut self) -> Option<Frame<PL_SIZE>> {
        trace!("Dequeueing frame.");
        self.dequeue()
    }
}

/// Network interface ID.
#[derive(Debug, Clone, Copy)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::spsc::Queue;

    #[test]
    fn given_queue_is_full_when_enqueue_then_drop_first_and_insert() {
        let mut q: Queue<Frame<10>, 5> = Queue::default();
        for i in 0..4u8 {
            let a = [i; 10];
            let frame = Frame::from_parts(VirtualLinkId(1), &a).unwrap();
            assert!(FrameQueue::enqueue_frame(&mut q, frame).is_ok());
        }
        assert_eq!(q.capacity(), q.len());
        assert_eq!(
            *q.peek().unwrap(),
            Frame::from_parts(VirtualLinkId(1), &[0; 10]).unwrap()
        );
        assert!(FrameQueue::enqueue_frame(
            &mut q,
            Frame::from_parts(VirtualLinkId(1), &[5; 10]).unwrap()
        )
        .is_ok());
        assert!(q
            .into_iter()
            .any(|x| *x == Frame::from_parts(VirtualLinkId(1), &[5; 10]).unwrap()));
        assert!(!q
            .into_iter()
            .any(|x| *x == Frame::from_parts(VirtualLinkId(1), &[4; 10]).unwrap()));
    }
}

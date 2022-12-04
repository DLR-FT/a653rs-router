//! Virtual links.

use crate::error::Error;
use crate::error::RouteError;
use crate::network::Frame;
use crate::network::PayloadSize;
use crate::prelude::ChannelId;
use crate::prelude::Queue;
use crate::routing::{PortIdIterator, RouteLookup};
use crate::shaper::QueueId;
use apex_rs::prelude::{
    ApexSamplingPortP4, MessageSize, SamplingPortDestination, SamplingPortSource, Validity,
};
use heapless::LinearMap;
use serde::{Deserialize, Serialize};

/// An ID of a virtual link.
///
/// Virtual links connect ports of different hypervisors and their contents may be transmitted over the network.
/// Each virtual link is a directed channel between a single source port and zero or more destination ports.
/// The virtual link ID is transmitted as an ID that identifies the virtual link to the network. For example the
/// id may be used as a VLAN tag id or ARINC 429 label words. If the size of the label used inside the network is
/// smaller than the 32 Bit, care must be taken by the system integrator that no IDs larger than the maximum size
/// are assigned. Implementations of the network interface layer should therefore cast this value to the desired
/// size that // is required by the underlying network protocol.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct VirtualLinkId(u32);

impl VirtualLinkId {
    /// The value of the VirtualLinkId.
    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl From<u32> for VirtualLinkId {
    fn from(val: u32) -> VirtualLinkId {
        VirtualLinkId(val)
    }
}

impl From<VirtualLinkId> for u32 {
    fn from(val: VirtualLinkId) -> u32 {
        val.0
    }
}

impl Default for VirtualLinkId {
    fn default() -> VirtualLinkId {
        VirtualLinkId(0)
    }
}

impl From<VirtualLinkId> for QueueId {
    fn from(val: VirtualLinkId) -> Self {
        Self::from(val.into_inner())
    }
}

/// Looks up the destinations of a virtual link from the router.
pub trait LookupVirtualLink<const PORTS: usize> {
    /// Gets the virtual link associated with the frame.
    fn get_virtual_link<'a>(
        &self,
        router: &'a dyn RouteLookup<PORTS>,
    ) -> Result<VirtualLinkDestinations<PORTS>, Error>;
}

// Router: Port -> VirtualLink
// VirtualLink: Frame -> ()

/// A bridge between the hypervisor ports and interfaces associated with a virtual link.
/// Stores information about a virtual link and
#[derive(Debug)]
pub struct VirtualLinkDestinations<const PORTS: usize> {
    queue: QueueId,
    ports: PortIdIterator<PORTS>,
}

impl<const PORTS: usize, const PL_SIZE: PayloadSize> LookupVirtualLink<PORTS> for Frame<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    fn get_virtual_link<'a>(
        &self,
        router: &'a dyn RouteLookup<PORTS>,
    ) -> Result<VirtualLinkDestinations<PORTS>, Error> {
        let ports = router.route_remote_input(&self.link)?;
        let vl = VirtualLinkDestinations::<PORTS> {
            queue: QueueId::from(self.link),
            ports,
        };
        Ok(vl)
    }
}

/// Receive a frame.
/// This should be implemented by things that can receive data from the network or from the hypervisor.
/// The ID of the frame should designate from which port / interface the data has been received.
pub trait ReceiveFrame {
    /// Receive a frame using the given frame as the destination.
    /// Returns the an `Ok` value of `frame` if the payload was received correctly.
    fn receive_frame<'a, const PL_SIZE: PayloadSize>(
        &self,
        frame: &'a mut Frame<PL_SIZE>,
    ) -> Result<&'a Frame<PL_SIZE>, Error>
    where
        [(); PL_SIZE as usize]:;
}

impl<const MSG_SIZE: MessageSize, H: ApexSamplingPortP4> ReceiveFrame
    for SamplingPortDestination<MSG_SIZE, H>
{
    fn receive_frame<'a, const PL_SIZE: PayloadSize>(
        &self,
        frame: &'a mut Frame<PL_SIZE>,
    ) -> Result<&'a Frame<PL_SIZE>, Error>
    where
        [(); PL_SIZE as usize]:,
    {
        let (valid, _) = self.receive(&mut frame.payload)?;
        if valid == Validity::Valid {
            Ok(frame)
        } else {
            Err(Error::InvalidData)
        }
    }
}

/// Looks up a sampling port source by its internal ID.
pub trait SamplingPortLookup<const MSG_SIZE: MessageSize, H: ApexSamplingPortP4> {
    /// Gets the sampling port source by the internal `id`.
    fn get_sampling_port_source<'a>(
        &'a self,
        id: &ChannelId,
    ) -> Option<&'a SamplingPortSource<MSG_SIZE, H>>
    where
        H: ApexSamplingPortP4;
}

impl<const MSG_SIZE: MessageSize, H: ApexSamplingPortP4, const PORTS: usize>
    SamplingPortLookup<MSG_SIZE, H>
    for LinearMap<ChannelId, SamplingPortSource<MSG_SIZE, H>, PORTS>
{
    fn get_sampling_port_source<'a>(
        &'a self,
        id: &ChannelId,
    ) -> Option<&'a SamplingPortSource<MSG_SIZE, H>>
    where
        H: ApexSamplingPortP4,
    {
        self.get(&id)
    }
}

/// Looks up a queue by its ID.
pub trait QueueLookup<const PL_SIZE: PayloadSize> {
    /// Gets a queue by its internal `id`.
    fn get_queue<'a>(&'a self, id: &'a QueueId) -> Option<&'a Queue<PL_SIZE>>;
}

impl<const PL_SIZE: PayloadSize, const QUEUES: usize> QueueLookup<PL_SIZE>
    for LinearMap<QueueId, Queue<PL_SIZE>, QUEUES>
{
    fn get_queue<'a>(&'a self, id: &'a QueueId) -> Option<&'a Queue<PL_SIZE>> {
        self.get(id)
    }
}

/// Forwards a frame to a set of port sources and network queues.
pub trait Forward {
    /// Forwards a frame to its destinations, which can be port sources or network queues.
    fn forward_sampling_port<'a, const PL_SIZE: PayloadSize, H: ApexSamplingPortP4>(
        self,
        frame: &Frame<PL_SIZE>,
        src_ports: &'a dyn SamplingPortLookup<PL_SIZE, H>,
        queues: &'a dyn QueueLookup<PL_SIZE>,
    ) -> Result<(), Error>
    where
        [(); PL_SIZE as usize]:;
}

impl<const PORTS: usize> Forward for VirtualLinkDestinations<PORTS> {
    fn forward_sampling_port<'a, const PL_SIZE: PayloadSize, H: ApexSamplingPortP4>(
        self,
        frame: &Frame<PL_SIZE>,
        src_ports: &'a dyn SamplingPortLookup<PL_SIZE, H>,
        queues: &'a dyn QueueLookup<PL_SIZE>,
    ) -> Result<(), Error>
    where
        [(); PL_SIZE as usize]:,
    {
        let ports = self.ports;
        for p in ports {
            let port = src_ports
                .get_sampling_port_source(&p)
                .ok_or(Error::InvalidRoute(RouteError::from(p)))?;

            port.send(&frame.payload)?; // TODO maybe collect errors and try every port?
        }

        // TODO enqueue frame by copying it
        let _queue = queues
            .get_queue(&self.queue)
            .ok_or(Error::InvalidRoute(RouteError::from(frame.link)))?;
        //queue.push(*frame);

        Ok(())
    }
}

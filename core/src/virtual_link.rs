//! Virtual links.

use crate::error::{Error, RouteError};
use crate::network::{Frame, PayloadSize, QueueLookup};
use crate::ports::SamplingPortLookup;
use crate::prelude::{Message, Shaper, Transmission};
use crate::routing::{PortIdIterator, RouteLookup};
use crate::shaper::QueueId;
use apex_rs::prelude::ApexSamplingPortP4;
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
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
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

impl From<VirtualLinkId> for QueueId {
    fn from(val: VirtualLinkId) -> Self {
        Self::from(val.into_inner())
    }
}

/// Looks up the destinations of a virtual link from the router.
pub trait LookupVirtualLink<const PORTS: usize> {
    /// Gets the virtual link associated with the frame.
    fn get_virtual_link(
        &self,
        router: &dyn RouteLookup<PORTS>,
    ) -> Result<VirtualLinkDestinations<PORTS>, Error>;
}

// Router: Port -> VirtualLink
// VirtualLink: Frame -> ()

/// A bridge between the hypervisor ports and interfaces associated with a virtual link.
/// Stores information about a virtual link and
#[derive(Debug)]
pub struct VirtualLinkDestinations<const PORTS: usize> {
    link: VirtualLinkId,
    queue: QueueId,
    ports: PortIdIterator<PORTS>,
}

impl<const PORTS: usize, const PL_SIZE: PayloadSize> LookupVirtualLink<PORTS> for Frame<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    fn get_virtual_link(
        &self,
        router: &dyn RouteLookup<PORTS>,
    ) -> Result<VirtualLinkDestinations<PORTS>, Error> {
        let ports = router.route_remote_input(&self.link)?;
        let vl = VirtualLinkDestinations::<PORTS> {
            link: self.link,
            queue: QueueId::from(self.link),
            ports,
        };
        Ok(vl)
    }
}

impl<const PORTS: usize, const PL_SIZE: PayloadSize> LookupVirtualLink<PORTS> for Message<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    fn get_virtual_link(
        &self,
        router: &dyn RouteLookup<PORTS>,
    ) -> Result<VirtualLinkDestinations<PORTS>, Error> {
        let vl_id = router.route_local_output(&self.port)?;
        let ports = router.route_remote_input(&vl_id)?;
        let vl = VirtualLinkDestinations::<PORTS> {
            link: vl_id,
            queue: QueueId::from(vl_id),
            ports,
        };
        Ok(vl)
    }
}

/// Forwards a frame to a set of port sources and network queues.
pub trait ForwardFrame {
    /// Forwards a frame to its destinations, which can be port sources or network queues.
    fn forward_frame<const PL_SIZE: PayloadSize, H: ApexSamplingPortP4>(
        self,
        frame: &Frame<PL_SIZE>,
        src_ports: &dyn SamplingPortLookup<PL_SIZE, H>,
    ) -> Result<(), Error>
    where
        [(); PL_SIZE as usize]:;
}

impl<const PORTS: usize> ForwardFrame for VirtualLinkDestinations<PORTS> {
    fn forward_frame<const PL_SIZE: PayloadSize, H: ApexSamplingPortP4>(
        self,
        frame: &Frame<PL_SIZE>,
        src_ports: &dyn SamplingPortLookup<PL_SIZE, H>,
    ) -> Result<(), Error>
    where
        [(); PL_SIZE as usize]:,
    {
        for src_port_id in self.ports {
            let port = src_ports
                .get_sampling_port_source(&src_port_id)
                .ok_or(Error::InvalidRoute(RouteError::from(src_port_id)))?;

            port.send(&frame.payload)?
        }
        Ok(())
    }
}

/// Forwards a message to a set of port sources and network queues.
pub trait ForwardMessage {
    /// Forwards a message to its destinations, which can be port sources or network queues.
    fn forward_message<const PL_SIZE: PayloadSize, H: ApexSamplingPortP4>(
        self,
        message: &Message<PL_SIZE>,
        src_ports: &dyn SamplingPortLookup<PL_SIZE, H>,
        queue: &mut dyn QueueLookup<PL_SIZE>,
        shaper: &mut dyn Shaper,
    ) -> Result<(), Error>
    where
        [(); PL_SIZE as usize]:;
}

impl<const PORTS: usize> ForwardMessage for VirtualLinkDestinations<PORTS> {
    fn forward_message<const PL_SIZE: PayloadSize, H: ApexSamplingPortP4>(
        self,
        message: &Message<PL_SIZE>,
        src_ports: &dyn SamplingPortLookup<PL_SIZE, H>,
        queues: &mut dyn QueueLookup<PL_SIZE>,
        shaper: &mut dyn Shaper,
    ) -> Result<(), Error>
    where
        [(); PL_SIZE as usize]:,
    {
        let ports = self.ports;
        for p in ports {
            let port = src_ports
                .get_sampling_port_source(&p)
                .ok_or(Error::InvalidRoute(RouteError::from(p)))?;

            port.send(&message.payload)?; // TODO maybe collect errors and try every port?
        }

        if let Some(q) = queues.get_queue(&self.queue) {
            let frame = Frame {
                link: self.link,
                payload: message.payload,
            };
            q.enqueue_frame(frame)?;
            shaper.request_transmission(&Transmission::for_frame(self.queue, &frame))?;
        }

        Ok(())
    }
}

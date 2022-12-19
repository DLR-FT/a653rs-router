//! Virtual links.

use crate::error::Error;
use crate::network::{Frame, PayloadSize};
use crate::prelude::{FrameQueue, Interface, Shaper, Transmission};
use crate::shaper::QueueId;
use crate::types::DataRate;
use apex_rs::prelude::{ApexSamplingPortP4, SamplingPortDestination, SamplingPortSource, Validity};
use core::fmt::{Debug, Display};
use core::time::Duration;
use heapless::spsc::Queue;
use heapless::Vec;

/// An ID of a virtual link.
///
/// Virtual links connect ports of different hypervisors and their contents may be transmitted over the network.
/// Each virtual link is a directed channel between a single source port and zero or more destination ports.
/// The virtual link ID is transmitted as an ID that identifies the virtual link to the network. For example the
/// id may be used as a VLAN tag id or ARINC 429 label words. If the size of the label used inside the network is
/// smaller than the 32 Bit, care must be taken by the system integrator that no IDs larger than the maximum size
/// are assigned. Implementations of the network interface layer should therefore cast this value to the desired
/// size that // is required by the underlying network protocol.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct VirtualLinkId(pub u32);

impl Display for VirtualLinkId {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl VirtualLinkId {
    /// Creates a virtual link id from an u32.
    pub const fn from_u32(val: u32) -> Self {
        Self(val)
    }

    /// The value of the VirtualLinkId.
    pub const fn into_inner(self) -> u32 {
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

/// A virtual link.
pub trait VirtualLink: Debug {
    /// Gets the VirtualLinkId.
    fn vl_id(&self) -> VirtualLinkId;

    /// Gets the queue id.
    fn queue_id(&self) -> Option<QueueId>;

    /// Receives frames from the hypervisor.
    fn receive_hypervisor(&mut self, shaper: &mut dyn Shaper) -> Result<(), Error>;

    /// Receives a message from the network and forwards it to the local ports.
    /// The contents of `buf` must already have been determined to belong to this virtual link (e.g. by comparing with the ID of this link).
    fn receive_network(&mut self, buf: &[u8]) -> Result<(), Error>;

    /// Sends frames from the queue of this virtual link to the network.
    /// The shaper is used for shaping the traffic emitted by the virtual link to the network.
    fn send_network(
        &mut self,
        interface: &dyn Interface,
        shaper: &mut dyn Shaper,
    ) -> Result<(), Error>;
}

/// The data of a virtual link.
#[derive(Debug)]
pub struct VirtualLinkData<
    const MTU: PayloadSize,
    const PORTS: usize,
    const MAX_QUEUE_LEN: usize,
    H: ApexSamplingPortP4,
> where
    [(); MTU as usize]:,
{
    id: VirtualLinkId,
    queue_id: Option<QueueId>,
    port_dst: Option<SamplingPortDestination<MTU, H>>,
    port_srcs: Vec<SamplingPortSource<MTU, H>, PORTS>,
    queue: Option<Queue<Frame<MTU>, MAX_QUEUE_LEN>>,
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4 + Debug,
    > VirtualLinkData<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    /// Creates a new virtual link.
    pub fn new(id: VirtualLinkId) -> Self {
        Self {
            id,
            queue_id: None,
            port_dst: None,
            port_srcs: Vec::default(),
            queue: None,
        }
    }

    /// Add a queue.
    pub fn queue(mut self, shaper: &mut dyn Shaper, share: DataRate) -> Self {
        if self.port_dst.is_some() {
            panic!("A virtual link may not both receive things from the network and receive things from the hypervisor.")
        }
        let queue_id = shaper.add_queue(share);
        self.queue_id = queue_id;
        self.queue = Some(Queue::default());
        self
    }

    /// Add a port destination.
    pub fn add_port_dst(&mut self, port_dst: SamplingPortDestination<MTU, H>) {
        if self.queue.is_some() {
            panic!("A virtual link may not both receive things from the network and receive things from the hypervisor.")
        }
        self.port_dst = Some(port_dst);
    }

    /// Adds a sampling port.
    pub fn add_port_src(&mut self, port_src: SamplingPortSource<MTU, H>) {
        self.port_srcs
            .push(port_src)
            .expect("Not enough free source port slots.");
    }
}

fn forward_to_network_queue<const MTU: PayloadSize, const MAX_QUEUE_LEN: usize>(
    queue_id: &QueueId,
    queue: &mut Queue<Frame<MTU>, MAX_QUEUE_LEN>,
    frame: Frame<MTU>,
    shaper: &mut dyn Shaper,
) -> Result<(), Error>
where
    [(); MTU as usize]:,
{
    let curr = queue.len() as u64;
    let next = queue.enqueue_frame(frame)?;
    if curr < next {
        let transmission = Transmission::new(*queue_id, Duration::ZERO, MTU);
        shaper.request_transmission(&transmission)
    } else {
        Ok(())
    }
}

fn send_network<const MTU: PayloadSize, const MAX_QUEUE_LEN: usize>(
    vl: &VirtualLinkId,
    queue_id: &QueueId,
    queue: &mut Queue<Frame<MTU>, MAX_QUEUE_LEN>,
    interface: &dyn Interface,
    shaper: &mut dyn Shaper,
) -> Result<Transmission, Error>
where
    [(); MTU as usize]:,
{
    let frame = queue.dequeue_frame();
    match frame {
        Some(frame) => {
            let buf = frame.as_slice();
            // Always remove credit from a queue. It is using its credit regardless of if the transmission was successful.
            let duration = match interface.send(vl, &buf) {
                Ok(dur) => dur,
                Err(dur) => dur,
            };
            let trans = Transmission::new(*queue_id, duration, buf.len() as u32);
            shaper.record_transmission(&trans)?;
            Ok(trans)
        }
        None => Err(Error::QueueEmpty),
    }
}

fn receive_sampling_port_valid<'a, const MTU: PayloadSize, H: ApexSamplingPortP4>(
    dst: &SamplingPortDestination<MTU, H>,
    buf: &'a mut [u8],
) -> Result<&'a [u8], Error> {
    let (valid, _) = dst.receive(buf)?;
    if valid == Validity::Invalid {
        return Err(Error::InvalidData);
    }
    Ok(buf)
}

fn forward_to_sources<const MTU: PayloadSize, const PORTS: usize, H: ApexSamplingPortP4>(
    srcs: &Vec<SamplingPortSource<MTU, H>, PORTS>,
    buf: &[u8],
) -> Result<(), Error> {
    if let Err(err) = srcs.iter().try_for_each(|p| p.send(buf)) {
        Err(Error::from(err))
    } else {
        Ok(())
    }
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4 + Debug,
    > VirtualLink for VirtualLinkData<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn receive_hypervisor(&mut self, shaper: &mut dyn Shaper) -> Result<(), Error> {
        if let Some(dst) = &mut self.port_dst {
            let mut buf = [0u8; MTU as usize];
            _ = receive_sampling_port_valid(dst, &mut buf)?;
            forward_to_sources(&self.port_srcs, &buf)?;
            if let Some(queue) = &mut self.queue {
                if let Some(id) = &mut self.queue_id {
                    return forward_to_network_queue(id, queue, Frame::from(buf), shaper);
                }
            }
        }
        Ok(())
    }

    fn send_network(
        &mut self,
        interface: &dyn Interface,
        shaper: &mut dyn Shaper,
    ) -> Result<(), Error> {
        if let Some(queue) = &mut self.queue {
            if let Some(id) = &mut self.queue_id {
                _ = send_network(&self.id, id, queue, interface, shaper)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn receive_network(&mut self, buf: &[u8]) -> Result<(), Error> {
        if self.port_dst.is_some() {
            // A VL may never receive things from both a local port and the network.
            // This means that another hypervisor is misconfigured to use one of the same
            // VLs as the local hypervisor.
            return Err(Error::InterfaceReceiveFail);
        }
        if buf.len() > MTU as usize {
            return Err(Error::InterfaceReceiveFail);
        }
        forward_to_sources(&self.port_srcs, buf)?;
        Ok(())
    }

    fn queue_id(&self) -> Option<QueueId> {
        self.queue_id
    }

    fn vl_id(&self) -> VirtualLinkId {
        self.id
    }
}

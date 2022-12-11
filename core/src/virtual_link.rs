//! Virtual links.

use crate::error::Error;
use crate::network::{Frame, PayloadSize};
use crate::prelude::{FrameQueue, Interface, Shaper, Transmission};
use crate::shaper::QueueId;
use apex_rs::prelude::{ApexSamplingPortP4, SamplingPortDestination, SamplingPortSource, Validity};
use bytesize::ByteSize;
use core::time::Duration;
use heapless::spsc::Queue;
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

//pub trait VirtualLink {
// TODO id field in enum
/// Receives a transmission from the hypervisor, if the virtual link is attached to a port destination.
pub trait ReceiveHypervisor {
    fn forward_hypervisor(&mut self, shaper: &mut dyn Shaper) -> Result<(), Error>;
}

pub trait ReceiveNetwork {
    /// Receives a message from the network and forwards it to the local ports.
    /// The contents of `buf` must already have been determined to belong to this virtual link (e.g. by comparing with the ID of this link).
    fn receive_network(&mut self, buf: &[u8]) -> Result<(), Error>;
}

pub trait SendNetwork {
    /// Sends frames from the queue of this virtual link to the network.
    /// The shaper is used for shaping the traffic emitted by the virtual link to the network.
    fn send_network(
        &mut self,
        interface: &mut dyn Interface,
        shaper: &mut dyn Shaper,
    ) -> Result<(), Error>;
}

pub trait QueueIdable {
    fn queue_id(&self) -> QueueId;
}

pub trait VirtualLinkIdable {
    fn vl_id(&self) -> VirtualLinkId;
}
pub trait LocalToNet: ReceiveHypervisor + SendNetwork + QueueIdable {}
pub trait LocalToNetAndLocal: ReceiveHypervisor + SendNetwork + QueueIdable {}
pub trait NetToLocal: ReceiveNetwork + VirtualLinkIdable {}

/// Different types of virtual links.
pub enum VirtualLink<'a> {
    LocalToLocal(&'a mut dyn ReceiveHypervisor),
    LocalToNet(&'a mut dyn LocalToNet),
    LocalToNetAndLocal(&'a mut dyn LocalToNetAndLocal),
    NetToLocal(&'a mut dyn NetToLocal),
}

#[derive(Debug)]
pub struct LocalToLocalVirtualLink<
    const MTU: PayloadSize,
    const PORTS: usize,
    H: ApexSamplingPortP4,
> where
    [(); MTU as usize]:,
{
    port_dst: SamplingPortDestination<MTU, H>,
    port_srcs: [SamplingPortSource<MTU, H>; PORTS],
}

fn receive_sampling_port_valid<'a, const MTU: PayloadSize, H: ApexSamplingPortP4>(
    dst: &SamplingPortDestination<MTU, H>,
    buf: &'a mut [u8],
) -> Result<&'a [u8], Error> {
    // TODO extract function
    let (valid, _) = dst.receive(buf)?;
    if valid == Validity::Invalid {
        return Err(Error::InvalidData);
    }
    Ok(buf)
}

fn forward_to_sources<const MTU: PayloadSize, const PORTS: usize, H: ApexSamplingPortP4>(
    srcs: &[SamplingPortSource<MTU, H>; PORTS],
    buf: &[u8],
) -> Result<(), Error> {
    if srcs.iter().map(|p| p.send(buf)).any(|e| e.is_err()) {
        Err(Error::SendFail)
    } else {
        Ok(())
    }
}

impl<const MTU: PayloadSize, const PORTS: usize, H: ApexSamplingPortP4> ReceiveHypervisor
    for LocalToLocalVirtualLink<MTU, PORTS, H>
where
    [(); MTU as usize]:,
{
    fn forward_hypervisor(&mut self, _shaper: &mut dyn Shaper) -> Result<(), Error> {
        let mut buf = [0u8; MTU as usize];
        let buf = receive_sampling_port_valid(&self.port_dst, &mut buf)?;
        forward_to_sources(&self.port_srcs, buf)
    }
}

#[derive(Debug)]
pub struct LocalToNetVirtualLink<
    const MTU: PayloadSize,
    const PORTS: usize,
    const MAX_QUEUE_LEN: usize,
    H: ApexSamplingPortP4,
> where
    [(); MTU as usize]:,
{
    id: VirtualLinkId,
    queue_id: QueueId,
    port_dst: SamplingPortDestination<MTU, H>,
    queue: Queue<Frame<MTU>, MAX_QUEUE_LEN>,
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
        let transmission = Transmission::new(*queue_id, Duration::ZERO, ByteSize::b(MTU as u64));
        shaper.request_transmission(&transmission)?;
    }
    Ok(())
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4,
    > ReceiveHypervisor for LocalToNetVirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn forward_hypervisor(&mut self, shaper: &mut dyn Shaper) -> Result<(), Error> {
        let mut buf = [0u8; MTU as usize];
        _ = receive_sampling_port_valid(&self.port_dst, &mut buf)?;
        forward_to_network_queue(&self.queue_id, &mut self.queue, Frame::from(buf), shaper)
    }
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4,
    > SendNetwork for LocalToNetVirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn send_network(
        &mut self,
        interface: &mut dyn Interface,
        shaper: &mut dyn Shaper,
    ) -> Result<(), Error> {
        _ = send_network(&self.id, &self.queue_id, &mut self.queue, interface, shaper)?;
        Ok(())
    }
}

fn send_network<const MTU: PayloadSize, const MAX_QUEUE_LEN: usize>(
    vl: &VirtualLinkId,
    queue_id: &QueueId,
    queue: &mut Queue<Frame<MTU>, MAX_QUEUE_LEN>,
    interface: &mut dyn Interface,
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
            let trans = Transmission::new(*queue_id, duration, ByteSize::b(buf.len() as u64));
            shaper.record_transmission(&trans)?;
            Ok(trans)
        }
        None => Err(Error::QueueEmpty),
    }
}

#[derive(Debug)]
pub struct LocalToLocalAndNetVirtualLink<
    const MTU: PayloadSize,
    const PORTS: usize,
    const MAX_QUEUE_LEN: usize,
    H: ApexSamplingPortP4,
> where
    [(); MTU as usize]:,
{
    id: VirtualLinkId,
    queue_id: QueueId,
    port_dst: SamplingPortDestination<MTU, H>,
    port_srcs: [SamplingPortSource<MTU, H>; PORTS],
    queue: Queue<Frame<MTU>, MAX_QUEUE_LEN>,
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4,
    > ReceiveHypervisor for LocalToLocalAndNetVirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn forward_hypervisor(&mut self, shaper: &mut dyn Shaper) -> Result<(), Error> {
        let mut buf = [0u8; MTU as usize];
        _ = receive_sampling_port_valid(&self.port_dst, &mut buf)?;
        forward_to_sources(&self.port_srcs, &buf)?;
        forward_to_network_queue(&self.queue_id, &mut self.queue, Frame::from(buf), shaper)?;
        Ok(())
    }
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4,
    > SendNetwork for LocalToLocalAndNetVirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn send_network(
        &mut self,
        interface: &mut dyn Interface,
        shaper: &mut dyn Shaper,
    ) -> Result<(), Error> {
        _ = send_network(&self.id, &self.queue_id, &mut self.queue, interface, shaper)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct NetToLocalVirtualLink<
    const MTU: PayloadSize,
    const PORTS: usize,
    const MAX_QUEUE_LEN: usize,
    H: ApexSamplingPortP4,
> where
    [(); MTU as usize]:,
{
    id: VirtualLinkId,
    port_srcs: [SamplingPortSource<MTU, H>; PORTS],
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4,
    > ReceiveNetwork for NetToLocalVirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn receive_network(&mut self, buf: &[u8]) -> Result<(), Error> {
        if buf.len() > MTU as usize {
            return Err(Error::ReceiveFail);
        }
        forward_to_sources(&self.port_srcs, buf)?;
        Ok(())
    }
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4,
    > QueueIdable for LocalToNetVirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn queue_id(&self) -> QueueId {
        self.queue_id
    }
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4,
    > QueueIdable for LocalToLocalAndNetVirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn queue_id(&self) -> QueueId {
        self.queue_id
    }
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const MAX_QUEUE_LEN: usize,
        H: ApexSamplingPortP4,
    > VirtualLinkIdable for NetToLocalVirtualLink<MTU, PORTS, MAX_QUEUE_LEN, H>
where
    [(); MTU as usize]:,
{
    fn vl_id(&self) -> VirtualLinkId {
        self.id
    }
}

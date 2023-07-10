//! Virtual links.

use crate::error::Error;
use crate::error::VirtualLinkError;
use crate::prelude::NetworkInterfaceId;
use crate::prelude::PayloadSize;
use a653rs::bindings::MessageRange;
use a653rs::prelude::ApexQueuingPortP4;
use a653rs::prelude::ApexSamplingPortP4;
use a653rs::prelude::Error as ApexError;
use a653rs::prelude::QueuingPortReceiver;
use a653rs::prelude::QueuingPortSender;
use a653rs::prelude::SamplingPortSource;
use a653rs::prelude::SystemTime;
use a653rs::prelude::{SamplingPortDestination, Validity};
use core::fmt::{Debug, Display};
use core::time::Duration;
use heapless::Vec;
use log::{debug, trace, warn};
use small_trace::*;

#[cfg(feature = "serde")]
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

/// A virtual link.
pub trait VirtualLink: Debug {
    /// Gets the VirtualLinkId.
    fn vl_id(&self) -> VirtualLinkId;

    /// Reads messages from local ports and writes them to local port and remote interfaces.
    fn read_local<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], Error>;

    /// Processes a message from a remote interface.
    fn process_remote(&self, buffer: &[u8]) -> Result<(), Error>;

    /// Checks if this virtual link connects to an interface.
    fn connects_to(&self, intf: &NetworkInterfaceId) -> bool;
}

/// Allows for modification of the ports and interfaces of a virtual link.
pub trait VirtualLinkData {
    /// Channel source / sender.
    type Source;
    /// Channel destination / receiver.
    type Destination;

    /// Adds a port destination / receiver.
    fn add_port_destination(&mut self, dst: Self::Destination);
    /// Adds a port source / sender.
    fn add_port_source(&mut self, src: Self::Source);
    /// Adds an interface as a destination of this virtual link.
    fn add_interface(&mut self, src: NetworkInterfaceId);
}

/// The data of a virtual link that forwards sampling ports.
#[derive(Debug)]
pub struct VirtualSamplingLink<
    const MTU: PayloadSize,
    const PORTS: usize,
    const IFS: usize,
    H: ApexSamplingPortP4,
> {
    id: VirtualLinkId,
    port_dst: Option<SamplingPortDestination<MTU, H>>,
    port_srcs: Vec<SamplingPortSource<MTU, H>, PORTS>,
    interfaces: Vec<NetworkInterfaceId, IFS>,
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const IFS: usize,
        H: ApexSamplingPortP4 + Debug,
    > VirtualSamplingLink<MTU, PORTS, IFS, H>
{
    /// Creates a new virtual link.
    pub fn new(id: VirtualLinkId) -> Self {
        Self {
            id,
            port_dst: None,
            port_srcs: Vec::default(),
            interfaces: Vec::default(),
        }
    }

    fn forward_to_local(&self, buffer: &[u8]) -> Result<(), Error> {
        if buffer.len() > MTU as usize {
            return Err(Error::VirtualLinkError(VirtualLinkError::MtuMismatch));
        }
        small_trace!(begin_forward_to_apex, self.id.0 as u16);
        let mut last_e: Option<Error> = None;
        for src in self.port_srcs.iter() {
            small_trace!(begin_apex_send, self.id.0 as u16);
            let res = src.send(buffer);
            small_trace!(end_apex_send, self.id.0 as u16);
            if let Err(e) = res {
                warn!("Failed to write to sampling port: {e:?}");
                last_e = Some(Error::PortSendFail(e));
            } else {
                trace!("Wrote to source: {buffer:?}")
            }
        }
        small_trace!(end_forward_to_apex, self.id.0 as u16);
        match last_e {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const IFS: usize,
        H: ApexSamplingPortP4 + Debug,
    > VirtualLinkData for VirtualSamplingLink<MTU, PORTS, IFS, H>
{
    type Source = SamplingPortSource<MTU, H>;
    type Destination = SamplingPortDestination<MTU, H>;

    fn add_port_destination(&mut self, dst: Self::Destination) {
        if !self.interfaces.is_empty() {
            panic!("A virtual link may not both receive things from the network and receive things from the hypervisor.")
        }
        trace!("Added port destination to virtual link {}", self.id);
        self.port_dst = Some(dst);
    }

    fn add_port_source(&mut self, src: Self::Source) {
        self.port_srcs
            .push(src)
            .expect("Not enough free source port slots.");
        trace!("Added port sources to virtual link {}", self.id);
    }

    /// Adds an interface to the destinations of this virtual link.
    fn add_interface(&mut self, interface: NetworkInterfaceId) {
        self.interfaces
            .push(interface)
            .expect("Not enough free interface slots");
        trace!("Added interface to virtual link: {}", self.id);
    }
}

impl<
        const MTU: PayloadSize,
        const PORTS: usize,
        const IFS: usize,
        H: ApexSamplingPortP4 + Debug,
    > VirtualLink for VirtualSamplingLink<MTU, PORTS, IFS, H>
{
    fn vl_id(&self) -> VirtualLinkId {
        self.id
    }

    fn process_remote(&self, buffer: &[u8]) -> Result<(), Error> {
        self.forward_to_local(buffer)
    }

    // TODO only call this if the scheduler says that it is this virtual link's turn.
    fn read_local<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], Error> {
        // Take the first data that is available
        if let Some(dst) = self.port_dst.as_ref() {
            small_trace!(begin_apex_receive, self.id.0 as u16);
            let res = dst.receive(buf);
            small_trace!(end_apex_receive, self.id.0 as u16);
            match res {
                Ok((val, data)) => {
                    if val == Validity::Invalid {
                        warn!("Reading invalid data from port");
                    } else {
                        self.forward_to_local(data)?;
                        return Ok(data);
                    }
                }
                Err(e) => {
                    warn!("Failed to receive data from port: {e:?}");
                }
            }
        }

        Err(Error::InvalidConfig)
    }

    fn connects_to(&self, intf: &NetworkInterfaceId) -> bool {
        self.interfaces.contains(intf)
    }
}

/// The data of a virtual link that forwards sampling ports.
#[derive(Debug)]
pub struct VirtualQueuingLink<
    const MTU: PayloadSize,
    const DEPTH: MessageRange,
    const PORTS: usize,
    const IFS: usize,
    H: ApexQueuingPortP4 + Debug,
> {
    id: VirtualLinkId,
    port_receiver: Option<QueuingPortReceiver<MTU, DEPTH, H>>,
    port_senders: Vec<QueuingPortSender<MTU, DEPTH, H>, PORTS>,
    interfaces: Vec<NetworkInterfaceId, IFS>,
}

impl<
        const MTU: PayloadSize,
        const DEPTH: MessageRange,
        const PORTS: usize,
        const IFS: usize,
        H: ApexQueuingPortP4 + Debug,
    > VirtualQueuingLink<MTU, DEPTH, PORTS, IFS, H>
{
    /// Creates a new virtual link.
    pub const fn new(id: VirtualLinkId) -> Self {
        Self {
            id,
            port_receiver: None,
            port_senders: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    fn forward_to_local(&self, buffer: &[u8]) -> Result<(), Error> {
        if buffer.len() > MTU as usize {
            return Err(Error::VirtualLinkError(VirtualLinkError::MtuMismatch));
        }
        small_trace!(begin_forward_to_apex, self.id.0 as u16);
        let mut last_e: Option<Error> = None;
        for src in self.port_senders.iter() {
            // TODO make configurable
            let timeout = SystemTime::Normal(Duration::from_micros(1));
            small_trace!(begin_apex_send, self.id.0 as u16);
            let res = src.send(buffer, timeout);
            small_trace!(end_apex_send, self.id.0 as u16);
            if let Err(e) = res {
                warn!("Failed to write to queuing port: {e:?}");
                last_e = Some(Error::PortSendFail(e));
            } else {
                trace!("Wrote to source: {buffer:?}")
            }
        }
        small_trace!(end_forward_to_apex, self.id.0 as u16);
        match last_e {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }
}

impl<
        const MTU: PayloadSize,
        const DEPTH: MessageRange,
        const PORTS: usize,
        const IFS: usize,
        H: ApexQueuingPortP4 + Debug,
    > VirtualLinkData for VirtualQueuingLink<MTU, DEPTH, PORTS, IFS, H>
{
    type Destination = QueuingPortReceiver<MTU, DEPTH, H>;
    type Source = QueuingPortSender<MTU, DEPTH, H>;

    /// Add a port destination.
    fn add_port_destination(&mut self, dst: Self::Destination) {
        if !self.interfaces.is_empty() {
            panic!("A virtual link may not both receive things from the network and receive things from the hypervisor.")
        }
        trace!("Added port destination to virtual link {}", self.id);
        self.port_receiver = Some(dst);
    }

    /// Adds a sampling port.
    fn add_port_source(&mut self, src: Self::Source) {
        self.port_senders
            .push(src)
            .expect("Not enough free source port slots.");
        trace!("Added port sources to virtual link {}", self.id);
    }

    /// Adds an interface to the destinations of this virtual link.
    fn add_interface(&mut self, interface: NetworkInterfaceId) {
        self.interfaces
            .push(interface)
            .expect("Not enough free interface slots");
        trace!("Added interface to virtual link: {}", self.id);
    }
}

impl<
        const MTU: PayloadSize,
        const DEPTH: MessageRange,
        const PORTS: usize,
        const IFS: usize,
        H: ApexQueuingPortP4 + Debug,
    > VirtualLink for VirtualQueuingLink<MTU, DEPTH, PORTS, IFS, H>
{
    fn vl_id(&self) -> VirtualLinkId {
        self.id
    }

    fn process_remote(&self, buffer: &[u8]) -> Result<(), Error> {
        self.forward_to_local(buffer)
    }

    fn read_local<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], Error> {
        trace!("Reading from local queuing port");
        if let Some(dst) = self.port_receiver.as_ref() {
            // TODO make configurable
            let timeout = SystemTime::Normal(Duration::from_micros(1));
            small_trace!(begin_apex_receive, self.id.0 as u16);
            let res = dst.receive(buffer, timeout);
            small_trace!(end_apex_receive, self.id.0 as u16);
            match res {
                Ok(data) => {
                    trace!("Received data from local queueing port");
                    if data.is_empty() {
                        warn!("Dropping empty message");
                    } else {
                        self.forward_to_local(data)?;
                        return Ok(data);
                    }
                }
                Err(ApexError::InvalidConfig) => {
                    warn!("Echo reply queue overflowed");
                }
                Err(ApexError::TimedOut) => {
                    debug!("Failed to receive data from port: TimedOut");
                }
                Err(e) => {
                    warn!("Failed to receive data from port: {e:?}");
                }
            }
        } else {
            trace!("No queuing port receivers for VL {}", self.id);
        }

        // TODO proper error
        Err(Error::InvalidConfig)
    }

    fn connects_to(&self, intf: &NetworkInterfaceId) -> bool {
        self.interfaces.contains(intf)
    }
}

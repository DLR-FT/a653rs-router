//! Virtual links.

use crate::error::Error;
use crate::error::VirtualLinkError;
use crate::prelude::NetworkInterfaceId;
use crate::prelude::PayloadSize;
use apex_rs::prelude::ApexSamplingPortP4;
use apex_rs::prelude::SamplingPortSource;
use apex_rs::prelude::{SamplingPortDestination, Validity};
use core::fmt::{Debug, Display};
use heapless::Vec;
use log::{trace, warn};

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

/// A virtual link.
pub trait VirtualLink: Debug {
    /// Gets the VirtualLinkId.
    fn vl_id(&self) -> VirtualLinkId;

    fn read_local<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], Error>;

    fn process_remote(&self, buffer: &[u8]) -> Result<(), Error>;
}

/// The data of a virtual link.
#[derive(Debug)]
pub struct VirtualLinkData<
    const MTU: PayloadSize,
    const PORTS: usize,
    const IFS: usize,
    H: ApexSamplingPortP4,
> where
    [(); MTU as usize]:,
{
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
    > VirtualLinkData<MTU, PORTS, IFS, H>
where
    [(); MTU as usize]:,
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

    /// Add a port destination.
    pub fn add_port_dst(&mut self, port_dst: SamplingPortDestination<MTU, H>) {
        if !self.interfaces.is_empty() {
            panic!("A virtual link may not both receive things from the network and receive things from the hypervisor.")
        }
        trace!("Added port destination to virtual link {}", self.id);
        self.port_dst = Some(port_dst);
    }

    /// Adds a sampling port.
    pub fn add_port_src(&mut self, port_src: SamplingPortSource<MTU, H>) {
        self.port_srcs
            .push(port_src)
            .expect("Not enough free source port slots.");
        trace!("Added port sources to virtual link {}", self.id);
    }

    /// Adds an interface to the destinations of this virtual link.
    pub fn add_interface(&mut self, interface: NetworkInterfaceId) {
        self.interfaces
            .push(interface)
            .expect("Not enough free interface slots");
        trace!("Added interface to virtual link: {}", self.id);
    }
}

fn receive_sampling_port_valid<'a, const MTU: PayloadSize, H: ApexSamplingPortP4>(
    dst: &SamplingPortDestination<MTU, H>,
    buf: &'a mut [u8],
) -> Result<&'a [u8], Error> {
    match dst.receive(buf) {
        Ok((Validity::Invalid, _)) => Err(Error::InvalidData),
        Err(e) => Err(Error::PortReceiveFail(e)),
        Ok((Validity::Valid, pl)) => Ok(pl),
    }
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
        const IFS: usize,
        H: ApexSamplingPortP4 + Debug,
    > VirtualLinkData<MTU, PORTS, IFS, H>
where
    [(); MTU as usize]:,
{
    fn forward_to_local(&self, buffer: &[u8]) -> Result<(), Error> {
        // TODO make sure only messages for this virtual link arrive here
        if buffer.len() > MTU as usize {
            return Err(Error::VirtualLinkError(VirtualLinkError::MtuMismatch));
        }

        let mut last_e: Option<Error> = None;

        for src in self.port_srcs.iter() {
            if let Err(e) = src.send(&buffer) {
                last_e = Some(Error::PortSendFail(e));
                warn!("Failed to write to {src:?}");
            } else {
                trace!("Wrote to source: {buffer:?}")
            }
        }

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
    > VirtualLink for VirtualLinkData<MTU, PORTS, IFS, H>
where
    [(); MTU as usize]:,
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
            match dst.receive(buf) {
                Ok((val, data)) => {
                    if val == Validity::Invalid {
                        warn!("Reading invalid data from port");
                    } else {
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
}

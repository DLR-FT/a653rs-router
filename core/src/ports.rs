// TODO link -> channel name -> port config -> port destination / source

/// An ID of a virtual link.
///
/// Virtual links connect ports of different hypervisors and their contents may be transmitted over the network.
/// Each virtual link is a directed channel between a single source port and zero or more destination ports.
/// The virtual link ID is transmitted as an ID that identifies the virtual link to the network. For example the
/// id may be used as a VLAN tag id or ARINC 429 label words. If the size of the label used inside the network is
/// smaller than the 32 Bit, care must be taken by the system integrator that no IDs larger than the maximum size
/// are assigned. Implementations of the network interface layer should therefore cast this value to the desired
/// size that // is required by the underlying network protocol.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

/// An ID of a hypervisor port.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ChannelId(u32);

impl ChannelId {
    /// Returns the name of the channel.
    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl From<u32> for ChannelId {
    fn from(val: u32) -> ChannelId {
        ChannelId(val)
    }
}

impl From<ChannelId> for u32 {
    fn from(val: ChannelId) -> u32 {
        val.0
    }
}

impl Default for ChannelId {
    fn default() -> ChannelId {
        ChannelId(0)
    }
}

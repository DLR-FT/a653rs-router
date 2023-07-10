use core::fmt::Display;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A data-rate in bit/s.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Ord, Eq, PartialEq, PartialOrd, Default)]
pub struct DataRate(pub u64);

impl DataRate {
    /// Constructs a data rate from a `u64` in bits/s.
    pub const fn b(bits: u64) -> Self {
        Self(bits)
    }

    /// Gets the bits/s as a `u64`.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// An ID of a virtual link.
///
/// Virtual links connect ports of different hypervisors and their contents may
/// be transmitted over the network. Each virtual link is a directed channel
/// between a single source port and zero or more destination ports. The virtual
/// link ID is transmitted as an ID that identifies the virtual link to the
/// network. For example the id may be used as a VLAN tag id or ARINC 429 label
/// words. If the size of the label used inside the network is smaller than the
/// 32 Bit, care must be taken by the system integrator that no IDs larger than
/// the maximum size are assigned. Implementations of the network interface
/// layer should therefore cast this value to the desired size that is
/// required by the underlying network protocol.
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

impl From<u16> for VirtualLinkId {
    fn from(val: u16) -> VirtualLinkId {
        VirtualLinkId(val as u32)
    }
}

impl From<VirtualLinkId> for u32 {
    fn from(val: VirtualLinkId) -> u32 {
        val.0
    }
}

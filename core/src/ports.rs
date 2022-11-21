use std::str::{FromStr, Utf8Error};

use apex_rs::{
    bindings::ApexName,
    prelude::{ApexUnsigned, Name},
};

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
pub type VirtualLinkId = u32;

/// The name of a channel between the network partition and another partition.
///
/// A name uniquely identifies a channel on a hypervisor. Each channel that is connected to the network partition
/// must belong to exactly one virtual link.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelName(Name);

impl ChannelName {
    /// Creates a new ChannelName.
    pub fn new(name: Name) -> Self {
        ChannelName(name)
    }

    /// Converts a channel name to a string slice.
    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        self.0.to_str()
    }

    /// Returns the name of the channel.
    pub fn into_inner(self) -> Name {
        self.0
    }
}

impl Default for ChannelName {
    fn default() -> Self {
        let invalid = ApexName::default();
        ChannelName(Name::new(invalid))
    }
}

impl FromStr for ChannelName {
    type Err = ApexUnsigned;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = Name::from_str(s)?;
        Ok(Self(name))
    }
}

impl From<ChannelName> for Name {
    fn from(val: ChannelName) -> Self {
        val.0
    }
}

impl From<Name> for ChannelName {
    fn from(val: Name) -> Self {
        ChannelName(val)
    }
}

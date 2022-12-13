use crate::prelude::PayloadSize;

// TODO
// - iterate over self.ports
// - lookup virtual link id of port
// - read port contents into frame
// - return iterator over new frames<'a>

/// An ID of a hypervisor port.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct PortId(u32);

impl PortId {
    /// Returns the name of the channel.
    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl From<u32> for PortId {
    fn from(val: u32) -> PortId {
        PortId(val)
    }
}

impl From<PortId> for u32 {
    fn from(val: PortId) -> u32 {
        val.0
    }
}

/// A message from the hypervisor, but annotated with the originating channel's id.
#[derive(Debug, Copy, Clone)]
pub struct Message<const PL_SIZE: PayloadSize>([u8; PL_SIZE as usize])
where
    [(); PL_SIZE as usize]:;

impl<const PL_SIZE: PayloadSize> Message<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    /// Gets the payload the message originated from.
    pub const fn into_inner(self) -> [u8; PL_SIZE as usize] {
        self.0
    }
}

impl<const PL_SIZE: PayloadSize> Default for Message<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    fn default() -> Self {
        Self([0u8; PL_SIZE as usize])
    }
}

impl<const PL_SIZE: PayloadSize> From<[u8; PL_SIZE as usize]> for Message<PL_SIZE>
where
    [u8; PL_SIZE as usize]:,
{
    fn from(val: [u8; PL_SIZE as usize]) -> Self {
        Self(val)
    }
}

use core::time::Duration;

use crate::error::Error;

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

/// A frame that is managed by the queue.
#[derive(Debug)]
pub struct Frame<const PL_SIZE: usize>(pub VirtualLinkId, pub [u8; PL_SIZE]);

impl<const PL_SIZE: usize> Frame<PL_SIZE> {
    /// The contents of a frame.
    pub const fn into_inner(self) -> (VirtualLinkId, [u8; PL_SIZE]) {
        (self.0, self.1)
    }

    /// The lengt of the payload of the frame.
    pub const fn len(&self) -> usize {
        PL_SIZE
    }
}

impl<const PL_SIZE: usize> From<[u8; PL_SIZE]> for Frame<PL_SIZE> {
    fn from(val: [u8; PL_SIZE]) -> Self {
        Self(VirtualLinkId::from(0), val)
    }
}

impl<const PL_SIZE: usize> From<Frame<PL_SIZE>> for [u8; PL_SIZE] {
    fn from(val: Frame<PL_SIZE>) -> Self {
        val.1
    }
}

/// A network interface.
pub(crate) trait Interface {
    /// Sends a frame with a payload of length PAYLOAD_SIZE.
    fn send<const PL_SIZE: usize>(&self, frame: Frame<PL_SIZE>) -> Result<Duration, Error>;

    /// Receives a frame with a payload of length PAYLOAD_SIZE using the supplied buffer.
    ///
    /// The buffer has to be large enough to contain the entire frame.
    fn receive<const PL_SIZE: usize>(
        &self,
        buffer: &mut Frame<PL_SIZE>,
    ) -> Result<&Frame<PL_SIZE>, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of_val;

    #[test]
    fn given_frame_when_len_then_len_of_payload() {
        let f = Frame(VirtualLinkId::from(0), [0u8; 10]);
        assert_eq!(10, size_of_val(&f.1));
    }
}

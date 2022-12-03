use core::time::Duration;

use crate::error::Error;
use crate::virtual_link::VirtualLinkId;

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

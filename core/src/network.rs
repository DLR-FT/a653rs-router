use core::time::Duration;

use crate::error::Error;
use crate::virtual_link::VirtualLinkId;

/// Size of a frame payload.
pub type PayloadSize = u32;

/// A frame that is managed by the queue.
#[derive(Debug)]
pub struct Frame<const PL_SIZE: PayloadSize>
where
    [u8; PL_SIZE as usize]:,
{
    /// Virtual link id.
    pub link: VirtualLinkId,
    /// Payload.
    pub payload: [u8; PL_SIZE as usize],
}

impl<const PL_SIZE: PayloadSize> Frame<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    /// Creates an empty frame for virtual link 0.
    pub fn default() -> Self {
        Self {
            link: VirtualLinkId::default(),
            payload: [0u8; PL_SIZE as usize],
        }
    }
    /// The contents of a frame.
    pub const fn into_inner(self) -> (VirtualLinkId, [u8; PL_SIZE as usize]) {
        (self.link, self.payload)
    }

    /// The lengt of the payload of the frame.
    pub const fn len(&self) -> usize {
        PL_SIZE as usize
    }
}

impl<const PL_SIZE: PayloadSize> From<[u8; PL_SIZE as usize]> for Frame<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    fn from(val: [u8; PL_SIZE as usize]) -> Self {
        Self {
            link: VirtualLinkId::from(0),
            payload: val,
        }
    }
}

/// A network interface.
pub(crate) trait Interface {
    /// Sends a frame with a payload of length PAYLOAD_SIZE.
    fn send<const PL_SIZE: PayloadSize>(&self, frame: Frame<PL_SIZE>) -> Result<Duration, Error>
    where
        [u8; PL_SIZE as usize]:;

    /// Receives a frame with a payload of length PAYLOAD_SIZE using the supplied buffer.
    ///
    /// The buffer has to be large enough to contain the entire frame.
    fn receive<const PL_SIZE: PayloadSize>(
        &self,
        buffer: &mut Frame<PL_SIZE>,
    ) -> Result<&Frame<PL_SIZE>, Error>
    where
        [u8; PL_SIZE as usize]:;
}

/// A queue for storing frames that are waiting to be transmitted.
#[derive(Debug)]
pub struct Queue<const PL_SIZE: PayloadSize>;

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of_val;

    #[test]
    fn given_frame_when_len_then_len_of_payload() {
        let f = Frame::<10> {
            link: VirtualLinkId::from(0),
            payload: [0u8; 10],
        };
        assert_eq!(10, size_of_val(&f.payload));
    }
}

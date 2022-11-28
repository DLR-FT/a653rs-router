use crate::error::Error;
use crate::ports::VirtualLinkId;

type FrameSize = usize;

/// A frame that is managed by the queue.
///
/// The length of a frame is not the length of the frame on the physical link, since
/// this frame is missing the network-specific headers.
#[derive(Debug, Clone)]
pub struct Frame<const PAYLOAD_SIZE: FrameSize> {
    /// ID for the link inside the NetworkInterface.
    link_id: VirtualLinkId,

    /// The payload of the frame to send.
    payload: [u8; PAYLOAD_SIZE],
}

impl<const PAYLOAD_SIZE: FrameSize> Frame<PAYLOAD_SIZE> {
    /// Creates a new Frame.
    pub const fn new(link_id: VirtualLinkId, payload: [u8; PAYLOAD_SIZE]) -> Self {
        Self { link_id, payload }
    }

    /// The length of a frame.
    pub const fn len(&self) -> usize {
        PAYLOAD_SIZE
    }

    pub const fn into_innter(self) -> (VirtualLinkId, [u8; PAYLOAD_SIZE]) {
        (self.link_id, self.payload)
    }
}

/// A network interface.
pub trait NetworkInterface {
    /// Sends a frame with a payload of length PAYLOAD_SIZE.
    fn send_frame<const PAYLOAD_SIZE: usize>(
        &self,
        frame: &Frame<PAYLOAD_SIZE>,
    ) -> Result<(), Error>;

    /// Receives a frame with a payload of length PAYLOAD_SIZE using the supplied buffer.
    ///
    /// The buffer has to be large enough to contain the entire frame.
    fn receive_frame<const PAYLOAD_SIZE: usize>(
        &self,
        buffer: &mut [u8; PAYLOAD_SIZE],
    ) -> Result<Frame<PAYLOAD_SIZE>, Error>;
}

#[derive(Debug, Clone)]
pub struct FakeNetworkInterface<const FAKE_DATA_SIZE: usize> {
    link_id: VirtualLinkId,
    fake_data: [u8; FAKE_DATA_SIZE],
}

impl<const FAKE_DATA_SIZE: usize> FakeNetworkInterface<FAKE_DATA_SIZE> {
    pub fn new(link_id: VirtualLinkId, data: [u8; FAKE_DATA_SIZE]) -> Self {
        Self {
            link_id,
            fake_data: data,
        }
    }
}

impl<const FAKE_DATA_SIZE: usize> NetworkInterface for FakeNetworkInterface<FAKE_DATA_SIZE> {
    fn send_frame<const PAYLOAD_SIZE: usize>(
        &self,
        _frame: &Frame<PAYLOAD_SIZE>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn receive_frame<const PAYLOAD_SIZE: usize>(
        &self,
        buffer: &mut [u8; PAYLOAD_SIZE],
    ) -> Result<Frame<PAYLOAD_SIZE>, Error> {
        if PAYLOAD_SIZE != FAKE_DATA_SIZE {
            Err(Error::InvalidData)
        } else {
            buffer.copy_from_slice(&self.fake_data);
            Ok(Frame::<PAYLOAD_SIZE> {
                link_id: self.link_id,
                payload: buffer.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_fake_frame_is_ok() {
        let link = VirtualLinkId::from(1);
        let fake = FakeNetworkInterface::new(link, [0, 1, 2, 3]);
        let frame = Frame::new(link, [0, 1, 3]);
        let result = fake.send_frame(&frame);
        assert!(result.is_ok());
    }

    #[test]
    fn receive_fake_frame_with_incorrect_size_is_not_ok() {
        let link = VirtualLinkId::from(1);
        const FAKE_DATA_SIZE: usize = 4;
        let fake = FakeNetworkInterface::new(link, [0u8, 1u8, 2u8, 3u8]);
        let mut buffer = [0u8; FAKE_DATA_SIZE];
        let result = fake.receive_frame(&mut buffer);
        assert!(result.is_ok());
    }
}

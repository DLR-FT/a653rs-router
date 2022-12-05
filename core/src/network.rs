use core::time::Duration;

use crate::error::Error;
use crate::virtual_link::VirtualLinkId;

/// Size of a frame payload.
pub type PayloadSize = u32;

/// A frame that is managed by the queue.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
    /// The contents of a frame.
    pub const fn into_inner(self) -> (VirtualLinkId, [u8; PL_SIZE as usize]) {
        (self.link, self.payload)
    }

    /// The lengt of the payload of the frame.
    pub const fn len(&self) -> usize {
        PL_SIZE as usize
    }

    /// True if the payload has length 0.
    pub const fn is_empty(&self) -> bool {
        PL_SIZE == 0
    }
}

impl<const PL_SIZE: PayloadSize> Default for Frame<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    /// Creates an empty frame for virtual link 0.
    fn default() -> Self {
        Self {
            link: VirtualLinkId::default(),
            payload: [0u8; PL_SIZE as usize],
        }
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
pub trait FrameQueue<const PL_SIZE: PayloadSize>
where
    [(); PL_SIZE as usize]:,
{
    /// Saves a frame to the queue to be written to the network later.
    /// If the underlying queue has no more free space, the oldest frame is dropped from the front
    /// and the new frame is inserted at the back.
    fn enqueue(&mut self, frame: Frame<PL_SIZE>) -> Result<(), Frame<PL_SIZE>>;

    /// Retrieves a frame from the queue to write it to the network.
    fn dequeue(&mut self) -> Option<Frame<PL_SIZE>>;
}

impl<const PL_SIZE: PayloadSize, const QUEUE_CAPACITY: usize> FrameQueue<PL_SIZE>
    for heapless::spsc::Queue<Frame<PL_SIZE>, QUEUE_CAPACITY>
where
    [(); PL_SIZE as usize]:,
{
    fn enqueue(&mut self, frame: Frame<PL_SIZE>) -> Result<(), Frame<PL_SIZE>> {
        let res = self.enqueue(frame);
        if res.is_err() {
            _ = self.dequeue();
            self.enqueue(frame)
        } else {
            res
        }
    }

    fn dequeue(&mut self) -> Option<Frame<PL_SIZE>> {
        self.dequeue()
    }
}

#[cfg(test)]
mod tests {
    use heapless::spsc::Queue;

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

    #[test]
    fn given_queue_is_full_when_enqueue_then_drop_first_and_insert() {
        let mut q: Queue<Frame<10>, 5> = Queue::default();
        for i in 0..4 {
            assert!(FrameQueue::enqueue(&mut q, Frame::from([i; 10])).is_ok());
        }
        assert_eq!(q.capacity(), q.len());
        assert_eq!(*q.peek().unwrap(), Frame::from([0; 10]));
        assert!(FrameQueue::enqueue(&mut q, Frame::from([5; 10])).is_ok());
        assert!(q.into_iter().any(|x| *x == Frame::from([5; 10])));
        assert!(!q.into_iter().any(|x| *x == Frame::from([4; 10])));
    }
}

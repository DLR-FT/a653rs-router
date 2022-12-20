use crate::error::Error;
use crate::types::DataRate;
use crate::virtual_link::VirtualLinkId;
use core::fmt::Debug;
use core::time::Duration;
use log::trace;

/// Size of a frame payload.
pub(crate) type PayloadSize = u32;

/// A frame that is managed by the queue.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct Frame<const PL_SIZE: PayloadSize>([u8; PL_SIZE as usize])
where
    [u8; PL_SIZE as usize]:;

impl<const PL_SIZE: PayloadSize> Frame<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    /// The contents of a frame.
    pub const fn as_slice(self) -> [u8; PL_SIZE as usize] {
        self.0
    }
}

impl<const PL_SIZE: PayloadSize> From<[u8; PL_SIZE as usize]> for Frame<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    fn from(val: [u8; PL_SIZE as usize]) -> Self {
        Self(val)
    }
}

/// A network interface.
pub trait Interface: Debug {
    /// Sends data to the interface.
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<Duration, Duration>;

    /// Receives data from the interface.
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<(VirtualLinkId, &'a [u8]), Error>;
}

/// A queue for storing frames that are waiting to be transmitted.
pub(crate) trait FrameQueue<const PL_SIZE: PayloadSize>
where
    [(); PL_SIZE as usize]:,
{
    /// Saves a frame to the queue to be written to the network later.
    /// If the underlying queue has no more free space, the oldest frame is dropped from the front
    /// and the new frame is inserted at the back.
    /// Returns the current size of the queue in bytes.
    fn enqueue_frame(&mut self, frame: Frame<PL_SIZE>) -> Result<u64, Error>;

    /// Retrieves a frame from the queue to write it to the network.
    fn dequeue_frame(&mut self) -> Option<Frame<PL_SIZE>>;
}

impl<const PL_SIZE: PayloadSize, const QUEUE_CAPACITY: usize> FrameQueue<PL_SIZE>
    for heapless::spsc::Queue<Frame<PL_SIZE>, QUEUE_CAPACITY>
where
    [(); PL_SIZE as usize]:,
{
    fn enqueue_frame(&mut self, frame: Frame<PL_SIZE>) -> Result<u64, Error> {
        let res = self.enqueue(frame);
        if res.is_err() {
            _ = self.dequeue();
            self.enqueue(frame).unwrap();
            trace!("Enqueued frame while overflowing a queue.");
            Ok(self.len() as u64 * PL_SIZE as u64)
        } else {
            trace!("Enqueued frame without overflowing a queue.");
            Ok(self.len() as u64 * PL_SIZE as u64)
        }
    }

    fn dequeue_frame(&mut self) -> Option<Frame<PL_SIZE>> {
        trace!("Dequeueing frame.");
        self.dequeue()
    }
}

/// Pseudo network interface.
///
/// The pseudo network interface will always emit the same frame when receiving frames from it.
/// The pseudo network interface will always assert that any frame was transmitted successfully
/// at the configured rate, but it discard the frame.
#[derive(Debug)]
pub struct PseudoInterface<'a> {
    vl: VirtualLinkId,
    buf: &'a [u8],
    rate: DataRate,
    mtu: u32,
}

impl<'a> PseudoInterface<'a> {
    /// Creates a new `PseudoInterface` that can receive `frame` and simulates transmission of frames with rate `rate`.
    pub fn new(vl: VirtualLinkId, buf: &'a [u8], rate: DataRate) -> Self {
        Self {
            vl,
            buf,
            rate,
            mtu: buf.len() as u32,
        }
    }
}

impl<'a> Interface for PseudoInterface<'a> {
    fn send(&self, _vl: &VirtualLinkId, _buf: &[u8]) -> Result<Duration, Duration> {
        let mtu = self.mtu as f64;
        let rate = self.rate.as_u64() as f64;
        let duration = mtu * 1_000_000_000.0 / rate;
        let duration = Duration::from_nanos(duration as u64);
        trace!("Sent frame to network, took {duration:#?}");
        Ok(duration)
    }

    fn receive<'b>(&self, buf: &'b mut [u8]) -> Result<(VirtualLinkId, &'b [u8]), Error> {
        if buf.len() < self.buf.len() {
            return Err(Error::InvalidData);
        }
        buf.clone_from_slice(&self.buf[0..buf.len()]);
        trace!(
            "Received frame with length {} bytes from network.",
            buf.len()
        );
        Ok((self.vl, buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::spsc::Queue;

    #[test]
    fn given_queue_is_full_when_enqueue_then_drop_first_and_insert() {
        let mut q: Queue<Frame<10>, 5> = Queue::default();
        for i in 0..4 {
            assert!(FrameQueue::enqueue_frame(&mut q, Frame::from([i; 10])).is_ok());
        }
        assert_eq!(q.capacity(), q.len());
        assert_eq!(*q.peek().unwrap(), Frame::from([0; 10]));
        assert!(FrameQueue::enqueue_frame(&mut q, Frame::from([5; 10])).is_ok());
        assert!(q.into_iter().any(|x| *x == Frame::from([5; 10])));
        assert!(!q.into_iter().any(|x| *x == Frame::from([4; 10])));
    }
}

use core::time::Duration;

use bytesize::ByteSize;

use crate::prelude::{Error, Frame, PayloadSize, QueueId, ReceiveFrame, SendFrame, Transmission};

/// Pseudo network interface.
///
/// The pseudo network interface will always emit the same frame when receiving frames from it.
/// The pseudo network interface will always assert that any frame was transmitted successfully
/// at the configured rate, but it discard the frame.
#[derive(Debug)]
pub struct PseudoInterface<const MTU: PayloadSize>
where
    [(); MTU as usize]:,
{
    frame: Frame<MTU>,
    rate: ByteSize,
    mtu: ByteSize,
}

impl<const PL_SIZE: PayloadSize> PseudoInterface<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    /// Creates a new `PseudoInterface` that can receive `frame` and simulates transmission of frames with rate `rate`.
    pub fn new(frame: Frame<PL_SIZE>, rate: ByteSize) -> Self {
        Self {
            frame,
            rate,
            mtu: ByteSize::b(PL_SIZE as u64),
        }
    }
}

impl<const MTU: PayloadSize> SendFrame for PseudoInterface<MTU>
where
    [(); MTU as usize]:,
{
    fn send_frame<const PL_SIZE: PayloadSize>(
        &self,
        queue: QueueId,
        frame: &Frame<PL_SIZE>,
    ) -> Result<Transmission, Transmission>
    where
        [(); PL_SIZE as usize]:,
    {
        let mtu = self.mtu.as_u64() as f64;
        let rate = self.rate.as_u64() as f64;
        let duration = mtu * 1_000_000_000.0 / rate;
        let duration = Duration::from_nanos(duration as u64);
        Ok(Transmission::new(
            queue,
            duration,
            ByteSize::b(frame.len() as u64),
        ))
    }
}

impl<const MTU: PayloadSize> ReceiveFrame for PseudoInterface<MTU>
where
    [(); MTU as usize]:,
{
    fn receive_frame<'a, const PL_SIZE: PayloadSize>(
        &self,
        frame: &'a mut Frame<PL_SIZE>,
    ) -> Result<&'a Frame<PL_SIZE>, Error>
    where
        [(); PL_SIZE as usize]:,
    {
        if PL_SIZE == MTU {
            frame.link = self.frame.link;
            frame
                .payload
                .clone_from_slice(&self.frame.payload[0..(PL_SIZE as usize)]);
            Ok(frame)
        } else {
            Err(Error::InvalidData)
        }
    }
}

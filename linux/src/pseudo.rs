use bytesize::ByteSize;
use core::time::Duration;
use log::trace;
use network_partition::prelude::{DataRate, Error, Interface, VirtualLinkId};

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
    mtu: ByteSize,
}

impl<'a> PseudoInterface<'a> {
    /// Creates a new `PseudoInterface` that can receive `frame` and simulates transmission of frames with rate `rate`.
    pub fn new(vl: VirtualLinkId, buf: &'a [u8], rate: DataRate) -> Self {
        Self {
            vl,
            buf,
            rate,
            mtu: ByteSize::b(buf.len() as u64),
        }
    }
}

impl<'a> Interface for PseudoInterface<'a> {
    fn send(&self, _vl: &VirtualLinkId, _buf: &[u8]) -> Result<Duration, Duration> {
        let mtu = self.mtu.as_u64() as f64;
        let rate = self.rate.as_u64() as f64;
        let duration = mtu * 1_000_000_000.0 / rate;
        let duration = Duration::from_nanos(duration as u64);
        Ok(duration)
    }

    fn receive<'b>(&self, buf: &'b mut [u8]) -> Result<(VirtualLinkId, &'b [u8]), Error> {
        if buf.len() < self.buf.len() {
            return Err(Error::InvalidData);
        }

        buf.clone_from_slice(&self.buf[0..buf.len()]);
        trace!("Frame received: {buf:?}");
        Ok((self.vl, buf))
    }
}

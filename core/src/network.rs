use core::time::Duration;

use crate::error::Error;

pub type FrameSize = usize;

/// A frame that is managed by the queue.
#[derive(Debug, PartialEq, Eq)]
pub struct Frame<'a>(&'a [u8]);

impl<'a> Frame<'a> {
    pub const fn into_inner(self) -> &'a [u8] {
        self.0
    }
}

impl<'a> From<&'a [u8]> for Frame<'a> {
    fn from(val: &'a [u8]) -> Self {
        Self(val)
    }
}

impl<'a> From<Frame<'a>> for &'a [u8] {
    fn from(val: Frame<'a>) -> Self {
        val.0
    }
}

/// A network interface.
pub trait Interface<const MTU: usize> {
    /// Sends a frame with a payload of length PAYLOAD_SIZE.
    fn send<'a>(&self, frame: Frame<'a>) -> Result<Duration, Error>;

    /// Receives a frame with a payload of length PAYLOAD_SIZE using the supplied buffer.
    ///
    /// The buffer has to be large enough to contain the entire frame.
    fn receive<'a>(&self, buffer: &mut Frame<'a>) -> Result<&'a Frame, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO
}

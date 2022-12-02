use core::time::Duration;

use crate::error::Error;

/// A frame that is managed by the queue.
#[derive(Debug, PartialEq, Eq)]
pub struct Frame<const PL_SIZE: usize>([u8; PL_SIZE]);

impl<const PL_SIZE: usize> Frame<PL_SIZE> {
    /// The contents of a frame.
    pub const fn into_inner(self) -> [u8; PL_SIZE] {
        self.0
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl<const PL_SIZE: usize> From<[u8; PL_SIZE]> for Frame<PL_SIZE> {
    fn from(val: [u8; PL_SIZE]) -> Self {
        Self(val)
    }
}

impl<const PL_SIZE: usize> From<Frame<PL_SIZE>> for [u8; PL_SIZE] {
    fn from(val: Frame<PL_SIZE>) -> Self {
        val.0
    }
}

/// A network interface.
pub trait Interface {
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

    // TODO
}

use crate::prelude::PayloadSize;

/// A message from the hypervisor, but annotated with the originating channel's id.
#[derive(Debug, Copy, Clone)]
pub(crate) struct Message<const PL_SIZE: PayloadSize>([u8; PL_SIZE as usize])
where
    [(); PL_SIZE as usize]:;

impl<const PL_SIZE: PayloadSize> Default for Message<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    fn default() -> Self {
        Self([0u8; PL_SIZE as usize])
    }
}

impl<const PL_SIZE: PayloadSize> From<[u8; PL_SIZE as usize]> for Message<PL_SIZE>
where
    [u8; PL_SIZE as usize]:,
{
    fn from(val: [u8; PL_SIZE as usize]) -> Self {
        Self(val)
    }
}

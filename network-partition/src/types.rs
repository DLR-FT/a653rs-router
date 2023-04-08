#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A data-rate in bit/s.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Ord, Eq, PartialEq, PartialOrd, Default)]
pub struct DataRate(pub u64);

impl DataRate {
    /// Constructs a data rate from a `u64` in bits/s.
    pub const fn b(bits: u64) -> Self {
        Self(bits)
    }

    /// Gets the bits/s as a `u64`.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

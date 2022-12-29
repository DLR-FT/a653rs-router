/// A data-rate in bit/s.
#[derive(Debug, Copy, Clone, Ord, Eq, PartialEq, PartialOrd)]
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

    /// Gets the data rate in bits/s as `f64`.
    pub const fn as_f64(self) -> f64 {
        self.0 as f64
    }
}

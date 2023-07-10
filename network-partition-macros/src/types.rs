use std::str::FromStr;

use bytesize::ByteSize;
use darling::FromMeta;

#[derive(Debug, Clone)]
pub struct WrappedByteSize(ByteSize);

impl WrappedByteSize {
    pub fn bytes(&self) -> u64 {
        self.0.as_u64()
    }
}

impl From<WrappedByteSize> for ByteSize {
    fn from(w: WrappedByteSize) -> Self {
        w.0
    }
}

impl FromMeta for WrappedByteSize {
    fn from_string(value: &str) -> darling::Result<Self> {
        match ByteSize::from_str(value) {
            Ok(s) => Ok(WrappedByteSize(s)),
            Err(e) => Err(darling::Error::unsupported_shape(&e)),
        }
    }
}

use std::{str::FromStr, time::Duration};

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

impl ToString for WrappedByteSize {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct WrappedDuration(Duration);

impl WrappedDuration {
    pub fn as_nanos(&self) -> u128 {
        self.0.as_nanos()
    }
}

impl From<WrappedDuration> for Duration {
    fn from(w: WrappedDuration) -> Self {
        w.0
    }
}

impl FromMeta for WrappedDuration {
    fn from_string(value: &str) -> darling::Result<Self> {
        match humantime::parse_duration(value) {
            Ok(s) => Ok(WrappedDuration(s)),
            Err(_e) => Err(darling::Error::unsupported_shape(value)),
        }
    }
}

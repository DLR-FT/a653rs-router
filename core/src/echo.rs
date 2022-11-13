//! Echo service
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
/// Echo message
pub struct Echo {
    /// A sequence number.
    pub sequence: u32,

    /// The time at which the message has been created.
    pub when_ms: u64,
}

use serde::{Deserialize, Serialize};

/// Echo message
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Echo {
    /// A sequence number.
    pub sequence: i32,

    /// The time at which the message has been created.
    pub when_ms: u64,
}

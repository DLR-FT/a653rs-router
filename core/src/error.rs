///! Error types
use serde::{Deserialize, Serialize};

// TODO more precise errors

/// General error type for this crate.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Error {
    /// Failed to send data to a port.
    SendFail,

    /// Failed to receive data from a port.
    ReceiveFail,

    /// Received invalid data.
    InvalidData,

    /// There is no route from the source port or virtual link.
    NoRoute,

    /// There is no link to the destination.
    NoLink,

    /// The route is invalid.
    InvalidRoute,

    /// An error occured while talking to the hypervisor.
    ApexError(apex_rs::prelude::Error),

    /// Insufficient credit
    InsufficientCredit,

    /// It has been tried to dequeue an item from an empty queue.
    QueueEmpty,

    /// A queue has no more free capacity.
    QueueOverflow,

    TransmitNotAllowed,

    BlockNotAllowed,

    /// An unspecified error.
    Unknown,
}

impl From<apex_rs::prelude::Error> for Error {
    fn from(val: apex_rs::prelude::Error) -> Self {
        Self::ApexError(val)
    }
}

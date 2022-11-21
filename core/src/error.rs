use serde::{Deserialize, Serialize};
use thiserror::Error;

/// TODO more precise errors

/// General error type for this crate.
#[derive(Error, Clone, Serialize, Deserialize, Debug)]
pub enum Error {
    /// Failed to send data to a port.
    #[error("failed to send data")]
    SendFail,

    /// Failed to receive data from a port.
    #[error("failed to receive data")]
    ReceiveFail,

    /// Received invalid data.
    #[error("received data was invalid")]
    InvalidData,

    /// There is no route from the source port or virtual link.
    #[error("no route")]
    NoRoute,

    /// There is no link to the destination.
    #[error("no link")]
    NoLink,

    /// The route is invalid.
    #[error("invalid route")]
    InvalidRoute,

    /// An error occured while talking to the hypervisor.
    #[error("APEX error")]
    ApexError(apex_rs::prelude::Error),

    /// An unspecified error.
    #[error("unknown routing error")]
    Unknown,
}

impl From<apex_rs::prelude::Error> for Error {
    fn from(val: apex_rs::prelude::Error) -> Self {
        Self::ApexError(val)
    }
}

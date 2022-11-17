use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Clone, Serialize, Deserialize, Debug)]
pub enum Error {
    #[error("failed to send data")]
    SendFail,

    #[error("failed to receive data")]
    ReceiveFail,

    #[error("the APEX encountered an error")]
    ApexError(apex_rs::prelude::Error),

    #[error("received data was invalid")]
    InvalidData,

    #[error("no route")]
    NoRoute,

    #[error("no link")]
    NoLink,

    #[error("invalid route")]
    InvalidRoute,

    #[error("unknown routing error")]
    Unknown,
}

impl From<apex_rs::prelude::Error> for Error {
    fn from(val: apex_rs::prelude::Error) -> Self {
        Self::ApexError(val)
    }
}

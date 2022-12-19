///! Error types
use crate::shaper::{QueueId, Transmission};
use apex_rs::prelude::Error as ApexError;

// TODO more precise errors

/// General error type for this crate.
#[derive(Clone, Debug)]
pub enum Error {
    /// Failed to send data to a port.
    PortSendFail(apex_rs::prelude::Error),

    /// Failed to receive data from a port.
    PortReceiveFail(apex_rs::prelude::Error),

    /// Failed to receive something from an interface.
    InterfaceReceiveFail,

    /// Received invalid data.
    InvalidData,

    /// It has been tried to dequeue an item from an empty queue.
    QueueEmpty,

    /// Transmission is not allowed at the time.
    TransmitNotAllowed,

    /// No such queue.
    NoSuchQueue(QueueId),

    /// Invalid transmission.
    InvalidTransmission(Transmission),

    /// An unspecified error.
    Unknown,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::PortSendFail(source) => write!(f, "Failed to send data: {source:?}"),
            Error::PortReceiveFail(source) => write!(f, "Failed to receive data: {source:?}"),
            Error::InterfaceReceiveFail => write!(f, "Failed to receive data from an interface"),
            Error::InvalidData => write!(f, "Received invalid data."),
            Error::QueueEmpty => write!(f, "Tried to dequeue an element of an empty queue."),
            Error::TransmitNotAllowed => write!(
                f,
                "Transmissions from this queue are not allowed at the moment."
            ),
            Error::InvalidTransmission(transmission) => {
                write!(f, "Invalid transmission: {transmission:?}")
            }
            Error::NoSuchQueue(q_id) => write!(f, "No such queue: {q_id}"),
            Error::Unknown => write!(f, "Unknown error"),
        }
    }
}

impl From<apex_rs::prelude::Error> for Error {
    fn from(err: ApexError) -> Self {
        match err {
            ApexError::ReadError(_) => Self::PortReceiveFail(err),
            ApexError::WriteError(_) => Self::PortSendFail(err),
            _ => Self::Unknown,
        }
    }
}

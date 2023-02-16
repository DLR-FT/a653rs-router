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
    InterfaceReceiveFail(InterfaceError),

    /// Interface failed to send data.
    InterfaceSendFail(InterfaceError),

    /// Failed to create interface.
    InterfaceCreationError(InterfaceError),

    /// Received invalid data.
    InvalidData,

    /// Invalid configuration for network partition
    InvalidConfig,

    /// It has been tried to dequeue an item from an empty queue.
    QueueEmpty,

    /// Enqueue failed
    EnqueueFailed,

    /// Transmission is not allowed at the time.
    TransmitNotAllowed,

    /// No such queue.
    NoSuchQueue(QueueId),

    /// Invalid transmission.
    InvalidTransmission(Transmission),

    /// An unspecified error.
    Unknown,
}

/// Inteface error type.
#[derive(Clone, Debug)]
pub enum InterfaceError {
    /// Insufficient buffer space
    InsufficientBuffer,
    /// No data available
    NoData,
    /// Invalid data received from interface
    InvalidData,
    /// Interface not found
    NotFound,
}

impl core::fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoData => write!(f, "No data available"),
            Self::InsufficientBuffer => write!(f, "Insufficient buffer space"),
            Self::InvalidData => write!(f, "Invalid data"),
            Self::NotFound => write!(f, "Interface not found"),
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::InterfaceCreationError(e) => write!(f, "Failed to create interface: {e}"),
            Error::PortSendFail(source) => write!(f, "Failed to send data: {source:?}"),
            Error::PortReceiveFail(source) => write!(f, "Failed to receive data: {source:?}"),
            Error::InterfaceReceiveFail(reason) => {
                write!(f, "Failed to receive data from an interface: {reason}")
            }
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
            Error::EnqueueFailed => write!(f, "Failed to enqueue a frame into queue"),
            Error::InvalidConfig => write!(f, "Invalid configuration"),
            Error::InterfaceSendFail(e) => write!(f, "Interface failed to send some data: {e}"),
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

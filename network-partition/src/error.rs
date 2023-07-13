//! Error types

use a653rs::prelude::Error as ApexError;
use core::fmt::{Display, Formatter};

// TODO more precise errors

/// General error type for this crate.
#[derive(Clone, Debug)]
pub enum Error {
    /// Failed to send data to a port.
    PortSendFail(a653rs::prelude::Error),

    /// Failed to receive data from a port.
    PortReceiveFail(a653rs::prelude::Error),

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

    /// An error that occured while processing a virtual link.
    VirtualLinkError(VirtualLinkError),

    /// Error while accessing the scheduler.
    IoScheduleError(IoScheduleError),

    /// An unspecified error.
    Unknown,
}

/// An error occureed while scheduling a virtual link.
#[derive(Clone, Debug)]
pub enum IoScheduleError {
    /// Failed to create a schedule
    CreationFailed,
}

impl Display for IoScheduleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CreationFailed => write!(f, "Failed to create the interface."),
        }
    }
}

#[derive(Clone, Debug)]
pub enum VirtualLinkError {
    /// A message was too long for this virtual link.
    MtuMismatch,
}

impl Display for VirtualLinkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MtuMismatch => write!(f, "A message was too long for this virtual link."),
        }
    }
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
    /// Sending failed
    SendFailed,
}

impl Display for InterfaceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoData => write!(f, "No data available"),
            Self::InsufficientBuffer => write!(f, "Insufficient buffer space"),
            Self::InvalidData => write!(f, "Invalid data"),
            Self::NotFound => write!(f, "Interface not found"),
            Self::SendFailed => write!(f, "Send failed"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::IoScheduleError(e) => write!(f, "Error while accessing the scheduler: {e}"),
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
            Error::EnqueueFailed => write!(f, "Failed to enqueue a frame into queue"),
            Error::InvalidConfig => write!(f, "Invalid configuration"),
            Error::InterfaceSendFail(e) => write!(f, "Interface failed to send some data: {e}"),
            Error::VirtualLinkError(e) => write!(f, "Virtual link encountered an error: {e}"),
            Error::Unknown => write!(f, "Unknown error"),
        }
    }
}

impl From<a653rs::prelude::Error> for Error {
    fn from(err: ApexError) -> Self {
        match err {
            ApexError::ReadError => Self::PortReceiveFail(err),
            ApexError::WriteError => Self::PortSendFail(err),
            _ => Self::Unknown,
        }
    }
}

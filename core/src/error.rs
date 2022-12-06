///! Error types
use serde::{Deserialize, Serialize};

use crate::prelude::{ChannelId, Frame, PayloadSize, VirtualLinkId};

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
    InvalidRoute(RouteError),

    /// An error occured while talking to the hypervisor.
    ApexError(apex_rs::prelude::Error),

    /// Insufficient credit
    InsufficientCredit,

    /// It has been tried to dequeue an item from an empty queue.
    QueueEmpty,

    /// A queue has no more free capacity.
    QueueOverflow,

    /// Transmission is not allowed at the time.
    TransmitNotAllowed,

    /// Failed to send a frame to the network.
    InterfaceSendFail,

    /// An unspecified error.
    Unknown,
}

impl From<apex_rs::prelude::Error> for Error {
    fn from(val: apex_rs::prelude::Error) -> Self {
        Self::ApexError(val)
    }
}

impl<const PL_SIZE: PayloadSize> From<Frame<PL_SIZE>> for Error
where
    [(); PL_SIZE as usize]:,
{
    fn from(_: Frame<PL_SIZE>) -> Self {
        Error::SendFail
    }
}

/// A routing error.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum RouteError {
    Local(ChannelId),
    Remote(VirtualLinkId),
}

impl From<ChannelId> for RouteError {
    fn from(val: ChannelId) -> Self {
        Self::Local(val)
    }
}

impl From<VirtualLinkId> for RouteError {
    fn from(val: VirtualLinkId) -> Self {
        Self::Remote(val)
    }
}

//! Error types

use core::fmt::{Display, Formatter};

use crate::{
    network::InterfaceError,
    reconfigure::CfgError,
    router::{PortError, RouteError},
    scheduler::ScheduleError,
};

// TODO more precise errors

/// General error type for this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// An issue with the route information.
    Route(RouteError),

    /// Communication error with the hypervisor's APEX.
    Port(PortError),

    /// Communication error with a network interface driver.
    Interface(InterfaceError),

    /// Invalid configuration for network partition
    Configuration(CfgError),

    /// Error while accessing the scheduler.
    Schedule(ScheduleError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Port(e) => write!(f, "Port error: {e:?}"),
            Error::Interface(e) => write!(f, "Operation on interface failed: {e:?}"),
            Error::Configuration(e) => write!(f, "Invalid configuration: {e:?}"),
            Error::Schedule(e) => write!(f, "Error while accessing the scheduler: {e}"),
            Error::Route(e) => write!(f, "Routing failed: {e:?}"),
        }
    }
}

impl From<RouteError> for Error {
    fn from(value: RouteError) -> Self {
        Error::Route(value)
    }
}

impl From<PortError> for Error {
    fn from(value: PortError) -> Self {
        Error::Port(value)
    }
}

impl From<InterfaceError> for Error {
    fn from(value: InterfaceError) -> Self {
        Error::Interface(value)
    }
}

impl From<CfgError> for Error {
    fn from(value: CfgError) -> Self {
        Error::Configuration(value)
    }
}

impl From<ScheduleError> for Error {
    fn from(value: ScheduleError) -> Self {
        Error::Schedule(value)
    }
}

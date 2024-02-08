//! Error types

use core::fmt::{Display, Formatter};

use crate::{
    config::RouterConfigError, network::InterfaceError, ports::PortError, process::ProcessError,
    router::RouteError, scheduler::ScheduleError,
};

/// General error type for this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// An issue with the route information.
    Route(RouteError),

    /// Communication error with the hypervisor's APEX.
    Port(PortError),

    /// Communication error with a network interface driver.
    Interface(InterfaceError),

    /// Invalid configuration for router.
    Configuration(RouterConfigError),

    /// Error while accessing the scheduler.
    Schedule(ScheduleError),

    /// Failed to create router process
    Process(ProcessError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Port(e) => write!(f, "Port error: {e:?}"),
            Error::Interface(e) => write!(f, "Operation on interface failed: {e:?}"),
            Error::Configuration(e) => write!(f, "Invalid configuration: {e:?}"),
            Error::Schedule(e) => write!(f, "Error while accessing the scheduler: {e}"),
            Error::Route(e) => write!(f, "Routing failed: {e:?}"),
            Error::Process(e) => write!(f, "Failed to create router process: {e:?}"),
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

impl From<RouterConfigError> for Error {
    fn from(value: RouterConfigError) -> Self {
        Error::Configuration(value)
    }
}

impl From<ScheduleError> for Error {
    fn from(value: ScheduleError) -> Self {
        Error::Schedule(value)
    }
}

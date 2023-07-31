//! Network partition for ARINC653 P1/P2/P4 based on apex-rs

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(incomplete_features)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results
)]

mod config;
pub mod error;
mod network;
mod reconfigure;
mod router;
mod run;
mod scheduler;
mod types;

pub use crate::run::*;

/// Standard Prelude to be used by network partition implementations (e.g.
/// network_partition_linux)
pub mod prelude {
    pub use crate::config::{BuilderResult, Config, ConfigResult, RouterConfigError};
    pub use crate::network::{
        CreateNetworkInterface, CreateNetworkInterfaceId, InterfaceConfig, NetworkInterface,
        NetworkInterfaceId, PayloadSize, PlatformNetworkInterface,
    };
    pub use crate::reconfigure::{Configurator, Resources};
    pub use crate::router::{Router, RouterInput, RouterOutput};
    pub use crate::scheduler::{DeadlineRrScheduler, Scheduler, TimeSource};
    pub use crate::types::*;
}

mod sealed {
    pub(crate) trait Sealed {}
}

#[cfg(feature = "macros")]
pub use network_partition_macros::*;

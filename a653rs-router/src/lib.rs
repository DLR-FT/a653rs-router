//! Message router and IO-partition for ARINC 653 P4 based on [`a653rs`](https://github.com/DLR-FT/a653rs).
//!
//! The router concept is explained in more detail in [*Towards Enabling Level 3A AI in Avionic Platforms*](https://doi.org/10.18420/se2023-ws-18).
//!
//! ## Using as a Partition
//!
//! See `a653rs-router-linux` for an example of how to run the router as a
//! partition.
//!
//! ## Configuration
//!
//! The router is configured using its configuration struct
//! [crate::prelude::RouterConfig], which is read at initialisation time.
//! The configuration has be provided by the partition code.
//! Common ways to do this are to read the configuration either
//! - from a YAML file or
//! - convert the YAML file to a `no_std` compatible format such as `postcard`
//!   and read the configuration data from a memory region.
//!
//! Configuration structs can also be dynamically constructed using
//! [crate::prelude::RouterConfigBuilder], which provides checks for every
//! construction step.
//!
//! ## Running the Router
//!
//! First, initialize the required resources for your router during partition
//! startup using the configuration struct. Then, inside the router process, run
//! the router.
//!
//! For a full example see [crate@a653rs-router-linux].
//!
//! ## Adding New Network Interface Implementations
//!
//! To add support for new network interface types, only
//! [`prelude::PlatformNetworkInterface`] and
//! [`prelude::CreateNetworkInterfaceId`] need to be implmeneted, one
//! providing the driver implementation for the interface type and the other
//! providing a way to create individual network interfaces of this type.
//!
//! To use the implementation, pass your new type as the `NetInf` generic
//! parameter value in the first code example.
//!
//! ## Required APEX Services
//!
//! The router requires the hypervisor to implement at least these traits:
//!
//! - `ApexTimeP4`
//! - `ApexPartitionP4`
//! - `ApexProcessP4`
//! - `ApexSamplingPortP4`
//! - `ApexQueuingPortP4`

#![no_std]
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
mod error;

#[macro_use]
mod macros;

mod network;
mod partition;
mod ports;
mod process;
mod router;
mod scheduler;
mod types;

/// Standard Prelude to be used by router partitions and network interface
/// implementations.
pub mod prelude {
    pub use crate::config::*;
    pub use crate::error::Error;
    pub use crate::network::{
        CreateNetworkInterfaceId, InterfaceConfig, InterfaceError, NetworkInterfaceId,
        PlatformNetworkInterface,
    };
    pub use crate::partition::RouterState;
    pub use crate::router::Router;
    pub use crate::scheduler::{InvalidTimeError, TimeSource};
    pub use crate::types::*;
}

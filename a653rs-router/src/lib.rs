//! Message router and IO-partition for ARINC 653 P4 based on [`a653rs`](https://github.com/DLR-FT/a653rs).
//!
//! The router concept is explained in more detail in [*Towards Enabling Level 3A AI in Avionic Platforms*](https://doi.org/10.18420/se2023-ws-18).
#![cfg_attr(
    feature = "macros",
    doc = r##"
## Using as a Partition

To create a partition containing the message router, it is recommended to
use the macros [`macro@router_config`], [`macro@run_router`] and
[`a653rs::partition`](https://docs.rs/a653rs_macros/latest/a653rs_macros/attr.partition.html)
that are available when requiring the `macros` feature
in both crates. For a full example see
[`a653rs_router_tests`](../a653rs_router_macros/index.html).
"##
)]
//! ## Configuration
//! The configuration of the router consists of two stages that depend on each
//! other. The compile-time configuration declares what inputs and outputs are
//! available to the router. The run-time configuration contains the routing
//! information that enables the router to forward messages between
//! attached partitions and between partitions and the network.
//!
//! ### Compile-Time Configuration
//!
//! The static part of the configuration can only be defined at compile time,
//! because it defines the maximum size of the data structures, such as the
//! route table. This has the advantage of providing a deterministic memory
//! consumption when the router is deployed. The size of these data-structures
//! is defined using const generics on types such as [`prelude::Router`].
//!
//! Each compile-time configuration provides a superset of the resources
//! required by all runtime configurations used in a specific deployment.
//!
//! ### Runtime Configuration
//!
//! The dynamic part of the configuration is defined at runtime. It is regularly
//! read from an external source that implements [`prelude::RouterInput`].
//! If the router detects a change to the configuration, it first checks that
//! the provided configuration is correct, and then attempts to reconfigure
//! itself.
//!
//! A configuration is correct only if all inputs and outputs that are
//! named by it are also part of the compile-time configuration. An individual
//! configuration may leave some statically configured inputs and outputs
//! unused.
//!
//! See [`prelude::Config`] for an example of the run-time configuration.
#![cfg_attr(
    feature = "serde",
    doc = r##"
## Running the Router
This crate defines a [run()] entry-point that continuously runs the
router. The entry-point is only available if the `serde` feature is enabled,
since it reads the serialized configuration from a [`prelude::RouterInput`].

```no_run
# #![no_std]
# use a653rs_router::prelude::*;
# use core::time::Duration;
#
# struct TimeSourceA;
# impl TimeSource for TimeSourceA {
#     fn get_time(&self) -> Result<Duration, InvalidTimeError> { todo!() }
# }
# struct RouterConfig;
# impl RouterInput for RouterConfig {
#     fn receive<'a>(
#        &self,
#        vl: &VirtualLinkId,
#        buf: &'a mut [u8],
#    ) -> Result<(VirtualLinkId, &'a [u8]), PortError> { todo!() }
# }
#
# fn main() {
#    let time_source = TimeSourceA {};
#    let router_config = RouterConfig {};
#    let mut scheduler = DeadlineRrScheduler::<2>::new();
#
let resources = Resources::<1, 1>::new();
// Add resources ...
a653rs_router::run::<1, 1, 1000>(
    &time_source as &dyn TimeSource,
    &router_config as &dyn RouterInput,
    resources,
    &mut scheduler as &mut dyn Scheduler,
)
# }
```
"##
)]
//! ## Adding New Network Interface Implementations
//!
//! To add support for new network interface types, only
//! [`prelude::PlatformNetworkInterface`] and
//! [`prelude::CreateNetworkInterfaceId`] need to be implmeneted, one
//! providing the driver implementation for the interface type and the other
//! providing a way to create individual network interfaces of this type.
//!
//! ## Required APEX Services
//!
//! The router requires the hypervisor to implement at least `ApexTimeP4`.

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
mod ports;
mod reconfigure;
mod router;
mod run;
mod scheduler;
mod types;

#[cfg(feature = "serde")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "serde")))]
pub use crate::run::*;

/// Standard Prelude to be used by router partitions and network interface
/// implementations.
pub mod prelude {
    pub use crate::config::{BuilderResult, Config, ConfigResult, RouterConfigError};
    pub use crate::error::Error;
    pub use crate::network::{
        CreateNetworkInterface, CreateNetworkInterfaceId, InterfaceConfig, InterfaceError,
        NetworkInterface, NetworkInterfaceId, PayloadSize, PlatformNetworkInterface,
    };
    pub use crate::reconfigure::{Configurator, Resources};
    pub use crate::router::{PortError, Router, RouterInput, RouterOutput};
    pub use crate::scheduler::{DeadlineRrScheduler, InvalidTimeError, Scheduler, TimeSource};
    pub use crate::types::*;
}

#[cfg(feature = "macros")]
pub use a653rs_router_macros::*;

#[cfg(feature = "partition")]
pub use a653rs_router_partition_macros::*;

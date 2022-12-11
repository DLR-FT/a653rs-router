//! Network partition for ARINC653 P1/P2/P4 based on apex-rs

#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
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
mod network;
mod partition;
mod ports;
mod shaper;
mod virtual_link;

/// Standard Prelude to be used by network partition implementations (e.g. network_partition_linux)
pub mod prelude {
    pub use crate::config::*;
    pub use crate::error::Error;
    pub use crate::network::*;
    pub use crate::partition::*;
    pub use crate::ports::*;
    pub use crate::shaper::*;
    pub use crate::virtual_link::*;
}

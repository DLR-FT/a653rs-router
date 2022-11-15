//! Network partition for ARINC653 P1/P2/P4 based on apex-rs

#![feature(generic_const_exprs)]
//#![no_std]
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

mod echo;
mod partition;
mod process;

/// Standard Prelude to be used by network partition implementations (e.g. network_partition_linux)
pub mod prelude {
    pub use crate::echo::{Echo, PortSampler};
    pub use crate::partition::{run, NetworkPartition};
}

//! Network partition for ARINC653 P1/P2/P4 based on apex-rs

#![no_std]
// Deny to compile things that are safe to deny
// https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html
#![deny(
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
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

/// Standard Prelude to be used by network partition implementations (e.g. network_partition_linux)
pub mod prelude {
    pub use crate::network_partition::Echo;
}

mod network_partition {
    use serde::{Deserialize, Serialize};

    /// Echo message
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct Echo {
        /// A sequence number.
        pub sequence: i32,

        /// The time at which the message has been created.
        pub when_ms: u64,
    }
}

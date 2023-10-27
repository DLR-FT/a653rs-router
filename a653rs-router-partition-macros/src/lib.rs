//! Macros for generating a modulethat can be used as a ARINC 653 partition that
//! executes the router.
//!
//! ## Example
//!
//! ```no_run
//! # #[macro_use] extern crate a653rs_router_partition_macros;
//! use a653rs_router_partition_macros::router_partition;
//!
//! #[router_partition(
//!     hypervisor = a653rs_linux::partition::ApexLinuxPartition,
//!     interface(
//!         name = "51234",
//!         kind = a653rs_router_linux::UdpNetworkInterface,
//!         destination = "127.0.0.1:51234",
//!         mtu = "1KB",
//!         rate = "100MB",
//!         source = "127.0.0.1:54234"
//!     ),
//!     inputs = 1,
//!     outputs = 1,
//!     mtu = "1.5KB",
//!     port(sampling_in(name = "[IGS]", msg_size = "1KB", refresh_period = "10s")),
//!     port(sampling_out(name = "CAS", msg_size = "1KB")),
//!     stack_size = "50MB",
//!     time_capacity = "5ms"
//! )]
//! mod my_router {}
//!
//! fn partition_entry_function() {
//!     my_router::run()
//! }
//! # fn main() {}
//! ```

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

use config::StaticRouterConfig;
use darling::{export::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod config;
mod router_partition;

/// Produces a module for the router partition with the configuration file that
/// is given as the first argument.
#[proc_macro_attribute]
pub fn router_partition(args: TokenStream, input: TokenStream) -> TokenStream {
    let module = parse_macro_input!(input as syn::ItemMod);
    let name = module.ident;

    NestedMeta::parse_meta_list(args.into())
        .map_err(darling::Error::from)
        .and_then(|meta_list| {
            StaticRouterConfig::from_list(&meta_list)
                .map(|config| router_partition::router_partition(name, config))
        })
        .unwrap_or_else(|e| e.write_errors())
        .into()
}

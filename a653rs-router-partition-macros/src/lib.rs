//! Macros for generating a modulethat can be used as a ARINC 653 partition that
//! executes the router.
//!
//! ## Example
//!
//! See [`a653rs_router_tests`](../a653rs_router_tests/index.html).

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

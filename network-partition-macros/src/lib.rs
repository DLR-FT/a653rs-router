//! Convenience macro for creating a network partition.
//!
//! For using this macro, a module is annotated with the [`network_partition()`]
//! attribute macro. Inside of this module are the interfaces that should be
//! serviced by the network partition.

mod attrs;
mod generate;
mod parse;
mod types;

use generate::GenerateStream;
use parse::{args::RunArgs, router::Router};
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemMod, TypePath};

#[proc_macro_attribute]
pub fn router_config(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let scheduler = parse_macro_input!(args as TypePath);
    let mut input = parse_macro_input!(input as ItemMod);
    Router::parse(&scheduler, &mut input)
        .and_then(|r| r.gen_stream(&mut input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn run_router(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as RunArgs);
    let router = &input.router;
    let router_config = &input.router_config;
    let inputs = &input.inputs.unwrap_or_else(|| parse_quote!([]));
    let outputs = &input.outputs.unwrap_or_else(|| parse_quote!([]));
    let time_source = &input.time_source;
    let stream = quote! {
        #router ::start::run(& #time_source , #router_config, #inputs, #outputs )
    };
    stream.into()
}

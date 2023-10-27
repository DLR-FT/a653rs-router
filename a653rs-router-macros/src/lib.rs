//! Convenience macros for
//! [`a653rs-router`](../a653rs_router/index.html).
//!
//! These macros are reexported as part of `a653rs-router`. To use them,
//! enable the `macros` feature for `a653rs-router`.
//!
//! To create the static part of the configuration for the router the
//! [`macro@router_config`] macro should be used. [`run_router!`] then runs this
//! router using the configuration.
//!
//! ## Example
//!
//! ```no_run
//! # #[macro_use] extern crate a653rs_router_macros;
//! #
//! use a653rs::partition;
//! use a653rs_router::router_config;
//!
//! #[router_config(a653rs_router::prelude::DeadlineRrScheduler)]
//! pub(crate) mod router {
//!     #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
//!     struct Limits;
//!
//!     #[interface(interface_type = a653rs_router_linux::UdpNetworkInterface)]
//!     #[interface(source = "tx", destination = "rx")]
//!     #[interface(rate = "10MB", mtu = "1.5KB")]
//!     struct NodeB;
//! }
//!
//! #[partition(a653rs_linux::partition::ApexLinuxPartition)]
//! mod router_partition {
//!     #[sampling_in(msg_size = "1KB", refresh_period = "1s")]
//!     struct EchoRequest;
//!
//!     #[sampling_out(msg_size = "1KB")]
//!     struct EchoReply;
//!
//!     #[sampling_in(name = "RouterConfig", refresh_period = "10s", msg_size = "1KB")]
//!     struct RouterConfig;
//!
//!     #[start(cold)]
//!     fn cold_start(ctx: start::Context) {
//!         warm_start(ctx);
//!     }
//!
//!     #[start(warm)]
//!     fn warm_start(mut ctx: start::Context) {
//!         ctx.create_echo_request().unwrap();
//!         ctx.create_echo_reply().unwrap();
//!         ctx.create_router_config().unwrap();
//!         ctx.create_aperiodic2().unwrap().start().unwrap();
//!     }
//!
//!     #[aperiodic(
//!         name = "ap2",
//!         time_capacity = "50ms",
//!         stack_size = "30KB",
//!         base_priority = 5,
//!         deadline = "Soft"
//!     )]
//!     fn aperiodic2(ctx: aperiodic2::Context) {
//!         let time_source = a653rs_linux::partition::ApexLinuxPartition {};
//!         let router_config = ctx.router_config.unwrap();
//!         let echo_request = ctx.echo_request.unwrap();
//!         let echo_reply = ctx.echo_reply.unwrap();
//!         run_router!(
//!             crate::router,
//!             time_source,
//!             router_config,
//!             [("EchoRequest", echo_request)],
//!             [("EchoReply", echo_reply)]
//!         );
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! ## `#[router_config(SCHEDULER)]`
//!
//! Defines the static part of the configuration of a router.
//! This takes a scheduler and a directly following module as arguments. The
//! macro modifies the module so that it contains the data structures for all
//! routing information and networking interfaces.
//!
//! - **SCHEDULER**: A type-path to a struct implementing
//!   `a653rs_router::prelude::Scheduler`. This is used for finding out when to
//!   forward a message from a virtual link.
//!
//! The generated module will contain the entry function `run` that takes a time
//! source, the port for passing in the runtime configuration and the hypervisor
//! ports used as inputs and outputs. For more see [`macro@run_router`].
//!
//! ### `#[limits(INPUTS, OUTPUTS, MTU)`
//! - **INPUTS**: The number of inputs to the router.
//! - **OUTPUTS**: The number of outputs from the router.
//! - **MTU**: The maximum buffer size and its unit.
//!
//! This macro is processed as part of [`macro@router_config`]. It defines the
//! memory resource limits of the router. For more information see the
//! documentation of [`a653rs-router`].
//!
//! ### `#[interface(INTERFACE_TYPE, SOURCE, DESTINATION, RATE, MTU)]`
//! - **INTERFACE_TYPE**: Type path to a struct implementing
//!   `a653rs_router::prelude::CreateNetworkInterface`.
//! - **SOURCE**: Name of the local source of this interface. For example,
//!   sources and destinations of an implementation for UDP sockets could take a
//!   [`String`] in the format of a `sockaddr`, an implementation for Ethernet
//!   may take VLAN tags.
//! - **RATE**: Data rate and its unit per second that can continually be
//!   acchieved using the interface. For example `10M` would be 10 mega bit per
//!   second.
//! - **MTU**: Maximum transfer unit or frame size of the interface.
//!
//! This macro is processed as part of [`macro@router_config`]. It defines the
//! configuration for an individual network interface. For more information see
//! the documentation of [`a653rs-router`].
//!
//! ### Caveats
//!
//! The network interfaces declared as part of the router configuration always
//! take the same set of parameters, but their interpretation is
//! implementation-dependent. The limit `mtu` should be chosen, so that
//! all individual messages from all hypervisor ports and interfaces fit this
//! value. The interfaces can count towards the limits for inputs and outputs,
//! depending on the configuration.
//!
//! ### Example
//!
//! ```no_run
//! # #[macro_use] extern crate a653rs_router;
//! #
//! use a653rs_router::router_config;
//!
//! #[router_config(a653rs_router::prelude::DeadlineRrScheduler)]
//! pub mod router {
//!     #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
//!     struct Limits;
//!
//!     #[interface(interface_type = a653rs_router_linux::UdpNetworkInterface)]
//!     #[interface(source = "tx", destination = "rx")]
//!     #[interface(rate = "10MB", mtu = "1.5KB")]
//!     struct NodeB;
//! }
//! # fn main() {}
//! ```
//!
//! ## `run_router!(ROUTER, TIME_SOURCE, RUNTIME_CONFIG, INPUTS, OUTPUTS)`
//!
//! Runs the router using its run function.
//!
//! This is a convenience wrapper around the `start::run` function generated by
//! [`macro@router_config`]. It should be called from the entry point of an
//! aperiodic process.
//!
//! - **ROUTER**: A module containing the static part of the router's
//!   configuration as defined by [`macro@router_config`].
//! - **TIME_SOURCE**: A source for the current system time. If your hypervisor
//!   implements `a653rs::prelude::ApexTimeP4Ext`, the hypervisor should be
//!   passed.
//! - **ROUTER_CONFIG**: A `a653rs_router::prelude::RouterInput` from which the
//!   router will source its runtime configuration. This can be a sampling port,
//!   queuing port, network interface or anything else that implements
//!   `RouterInput`.
//! - **INPUTS**: An array containing tuples of the names of inputs and outputs
//!   and references to the structs they can be accessed by. This can be
//!   anything implementing `a653rs_router::prelude::RouterInput`.
//!   `a653rs-router` defines implementations for sampling and queuing ports and
//!   `a653rs_router::prelude::NetworkInterface`.
//! - **OUTPUTS**: An array of the form of the previous argument, but containing
//!   elements of `a653rs_router::prelude::RouterOutput`.
//!
//! The outputs or the inputs and outputs may be left unspecified, in which case
//! they are presumed to be empty. The arrays need to be the exact same size as
//! specified using the `inputs` and `outputs` limits in
//! [`macro@router_config`]. This is to avoid reserving unused memory for the
//! router or missing inputs and outputs that are used in the runtime
//! configuration.
//!
//! See the module-level documentation of `a653rs-router` for more.

mod attrs;
mod generate;
mod parse;

use generate::GenerateStream;
use parse::{args::RunArgs, router::Router};
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemMod};

/// See the [module-level documentation](mod@self).
#[proc_macro_attribute]
pub fn router_config(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let scheduler = parse_macro_input!(args as syn::Path);
    let mut input = parse_macro_input!(input as ItemMod);
    Router::parse(&scheduler, &mut input)
        .and_then(|r| r.gen_stream(&mut input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// See the [module-level documentation](mod@self).
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

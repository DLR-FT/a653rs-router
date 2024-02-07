#![no_std]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

#[macro_use]
mod macros;
mod network;

pub use network::UartNetworkInterface;

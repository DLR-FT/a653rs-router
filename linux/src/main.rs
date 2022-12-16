#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod pseudo;

include!(concat!(env!("OUT_DIR"), "/config.rs"));

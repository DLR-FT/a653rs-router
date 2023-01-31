#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

extern "C" {
    pub fn XalPrintf(fmt: *const u8, ...) -> i32;
}

include!(concat!(env!("OUT_DIR"), "/np.rs"));

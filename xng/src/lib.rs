#![no_std]

extern "C" {
    pub fn XalPrintf(fmt: *const u8, ...) -> i32;
}

pub mod network;

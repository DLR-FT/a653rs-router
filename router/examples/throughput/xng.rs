#![no_std]

#[cfg(feature = "xng")]
extern crate a653rs_xng;

#[cfg(feature = "xng")]
#[no_mangle]
pub extern "C" fn main() {
    router::run()
}

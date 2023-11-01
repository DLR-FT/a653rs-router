#![no_std]

#[cfg(feature = "xng")]
#[no_mangle]
pub extern "C" fn main() {
    configurator::run()
}

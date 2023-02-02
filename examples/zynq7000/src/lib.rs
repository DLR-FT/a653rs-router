#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_rs::prelude::ApexTimeP1Ext;
use apex_rs_xng::apex::XngHypervisor;
use log::{Level, LevelFilter, Metadata, Record};
use network_partition_xng::network::UartSerial;

include!(concat!(env!("OUT_DIR"), "/np.rs"));

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn main() {
    loop {
        let buf = b"Hello, World!\0";
        let res = UartSerial::platform_interface_send_unchecked(
            NetworkInterfaceId(0),
            VirtualLinkId(1),
            buf,
        );
        XngHypervisor::timed_wait(Duration::from_millis(20));
    }

    //NetworkPartition.run();
}

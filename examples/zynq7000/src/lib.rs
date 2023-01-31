#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_rs::prelude::ApexTimeP1Ext;
use apex_rs_xng::apex::XngHypervisor;
use log::{Level, LevelFilter, Metadata, Record};
use network_partition_xng::network::UartSerial;
use network_partition_xng::XalPrintf;

include!(concat!(env!("OUT_DIR"), "/np.rs"));

struct XalLogger;

impl log::Log for XalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() < log::max_level()
    }

    fn log(&self, record: &Record) {
        unsafe { XalPrintf(b"AAAAAAAAA\n\0".as_ptr()) };

        // if self.enabled(record.metadata()) {
        // let mut buf = [0u8; 100];
        // let level = record.level();
        //if let Some(args) = record.args().as_str() {
        //let mut i = 0usize;
        // for (b, a) in buf[..98].iter_mut().zip(args.as_bytes()) {
        //     *b = *a;
        //     i += 1;
        // }
        //buf[i] = 0x0;
        //unsafe { XalPrintf(b"%s\n\0".as_ptr(), buf.as_ptr()) };
        //}
        //unsafe { XalPrintf(b"[%s] - [%s]\n\0".as_ptr(), level.as_str().char_indices };
        // }
    }

    fn flush(&self) {}
}

static LOGGER: XalLogger = XalLogger;

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn main() {
    //unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    //log::set_max_level(LevelFilter::Trace);

    for i in 0..10 {
        unsafe { XalPrintf(b"Looping\n\0".as_ptr()) };
        //trace!("This is trace...");
        //loop {}
        let buf = b"Hello, World!";
        let res = UartSerial::platform_interface_send_unchecked(
            NetworkInterfaceId(0),
            VirtualLinkId(1),
            buf,
        );
        XngHypervisor::timed_wait(Duration::from_millis(20));
    }

    //NetworkPartition.run();
}

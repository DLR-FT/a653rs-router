#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use apex_rs_linux::partition::ApexLogger;

include!(concat!(env!("OUT_DIR"), "/np-client.rs"));

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();
    NetworkPartition.run();
}

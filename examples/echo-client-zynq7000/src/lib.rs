#![no_std]
#![feature(generic_const_exprs)]
#![feature(int_roundings)]
#![allow(incomplete_features)]

use apex_echo::client::*;
use apex_rs::prelude::*;
use apex_rs_xng::apex::XngHypervisor;
use once_cell::unsync::OnceCell;

const ECHO_SIZE: MessageSize = 1000;

static mut SENDER: OnceCell<SamplingPortSource<ECHO_SIZE, XngHypervisor>> = OnceCell::new();
static mut RECEIVER: OnceCell<SamplingPortDestination<ECHO_SIZE, XngHypervisor>> = OnceCell::new();

#[no_mangle]
pub extern "C" fn main() {
    let partition = PeriodicEchoPartition::new(
        unsafe { &SENDER },
        unsafe { &RECEIVER },
        entry_point_periodic,
        entry_point_aperiodic,
    );
    partition.run()
}

extern "C" fn entry_point_periodic() {
    EchoSenderProcess::run(unsafe { SENDER.get_mut().unwrap() });
}

extern "C" fn entry_point_aperiodic() {
    EchoReceiverProcess::run(unsafe { RECEIVER.get_mut().unwrap() });
}

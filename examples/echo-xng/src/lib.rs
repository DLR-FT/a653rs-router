#![no_std]

use core::ptr::addr_of_mut;

use a653rs::{
    bindings::{ApexPartitionP4, OperatingMode},
    prelude::*,
};
use a653rs_xng::apex::XngHypervisor;
use echo::*;
use log::*;

static mut ECHO_RECEIVER_SAMPLING: Option<SamplingPortDestination<ECHO_SIZE, XngHypervisor>> = None;

static mut ECHO_SENDER_SAMPLING: Option<SamplingPortSource<ECHO_SIZE, XngHypervisor>> = None;

static mut ECHO_RECEIVER_QUEUING: Option<
    QueuingPortReceiver<ECHO_SIZE, ECHO_QUEUE_SIZE, XngHypervisor>,
> = None;

static mut ECHO_SENDER_QUEUING: Option<
    QueuingPortSender<ECHO_SIZE, ECHO_QUEUE_SIZE, XngHypervisor>,
> = None;

extern "C" fn client_send_sampling() {
    echo::run_echo_sampling_sender(unsafe { ECHO_SENDER_SAMPLING.as_ref().unwrap() })
}

extern "C" fn client_receive_sampling() {
    run_echo_sampling_receiver(unsafe { ECHO_RECEIVER_SAMPLING.as_ref().unwrap() })
}

extern "C" fn server_sampling() {
    run_echo_sampling_server(unsafe { ECHO_SENDER_SAMPLING.as_ref().unwrap() }, unsafe {
        ECHO_RECEIVER_SAMPLING.as_ref().unwrap()
    })
}

extern "C" fn client_send_queuing() {
    run_echo_queuing_sender(unsafe { ECHO_SENDER_QUEUING.as_ref().unwrap() })
}

extern "C" fn client_receive_queuing() {
    run_echo_queuing_receiver(unsafe { ECHO_RECEIVER_QUEUING.as_ref().unwrap() })
}

extern "C" fn server_queuing() {
    run_echo_queuing_server(unsafe { ECHO_SENDER_QUEUING.as_ref().unwrap() }, unsafe {
        ECHO_RECEIVER_QUEUING.as_ref().unwrap()
    })
}

struct Echo;

impl Partition<XngHypervisor> for Echo {
    #[allow(clippy::deref_addrof)]
    fn cold_start(&self, ctx: &mut StartContext<XngHypervisor>) {
        trace!("Running echo cold_start");
        cold_start_sampling_queuing(
            ctx,
            unsafe { &mut *(addr_of_mut!(ECHO_SENDER_QUEUING)) },
            unsafe { &mut *(addr_of_mut!(ECHO_RECEIVER_QUEUING)) },
            unsafe { &mut *(addr_of_mut!(ECHO_SENDER_SAMPLING)) },
            unsafe { &mut *(addr_of_mut!(ECHO_RECEIVER_SAMPLING)) },
            &(EchoEntryFunctions {
                client_send_sampling,
                client_receive_sampling,
                server_sampling,
                client_send_queuing,
                client_receive_queuing,
                server_queuing,
            }),
        );
        <XngHypervisor as ApexPartitionP4>::set_partition_mode(OperatingMode::Normal).unwrap();
    }

    fn warm_start(&self, ctx: &mut StartContext<XngHypervisor>) {
        self.cold_start(ctx)
    }
}

static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;

#[cfg(not(test))]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn main() {
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(log::LevelFilter::Info);
    trace!("Running echo main");
    Echo.run()
}

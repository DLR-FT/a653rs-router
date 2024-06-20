use std::ptr::addr_of_mut;

use a653rs::{bindings::ApexPartitionP4, prelude::*};
use a653rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use echo::*;
use log::{info, trace, LevelFilter};

static mut ECHO_RECEIVER_SAMPLING: Option<
    ConstSamplingPortDestination<ECHO_SIZE, ApexLinuxPartition>,
> = None;
static mut ECHO_SENDER_SAMPLING: Option<ConstSamplingPortSource<ECHO_SIZE, ApexLinuxPartition>> =
    None;

// Not supported by a653rs-linux-hypervisor

static mut ECHO_RECEIVER_QUEUING: Option<
    ConstQueuingPortReceiver<ECHO_SIZE, ECHO_QUEUE_SIZE, ApexLinuxPartition>,
> = None;
static mut ECHO_SENDER_QUEUING: Option<
    ConstQueuingPortSender<ECHO_SIZE, ECHO_QUEUE_SIZE, ApexLinuxPartition>,
> = None;

extern "C" fn client_send_sampling() {
    echo::run_echo_sampling_sender(unsafe { ECHO_SENDER_SAMPLING.as_ref().unwrap() })
}

extern "C" fn client_receive_sampling() {
    echo::run_echo_sampling_receiver(unsafe { ECHO_RECEIVER_SAMPLING.as_ref().unwrap() })
}

extern "C" fn server_sampling() {
    echo::run_echo_sampling_server(unsafe { ECHO_SENDER_SAMPLING.as_ref().unwrap() }, unsafe {
        ECHO_RECEIVER_SAMPLING.as_ref().unwrap()
    })
}

extern "C" fn client_send_queuing() {
    echo::run_echo_queuing_sender(unsafe { ECHO_SENDER_QUEUING.as_ref().unwrap() })
}

extern "C" fn client_receive_queuing() {
    echo::run_echo_queuing_receiver(unsafe { ECHO_RECEIVER_QUEUING.as_ref().unwrap() })
}

extern "C" fn server_queuing() {
    echo::run_echo_queuing_server(unsafe { ECHO_SENDER_QUEUING.as_ref().unwrap() }, unsafe {
        ECHO_RECEIVER_QUEUING.as_ref().unwrap()
    })
}

const ECHO_ENTRY_FUNCTIONS: EchoEntryFunctions = EchoEntryFunctions {
    client_send_sampling,
    client_receive_sampling,
    server_sampling,
    client_send_queuing,
    client_receive_queuing,
    server_queuing,
};

struct Echo;

#[allow(clippy::deref_addrof)]
impl Partition<ApexLinuxPartition> for Echo {
    fn cold_start(&self, ctx: &mut StartContext<ApexLinuxPartition>) {
        cold_start_sampling_queuing(
            ctx,
            unsafe { &mut *addr_of_mut!(ECHO_SENDER_QUEUING) },
            unsafe { &mut *addr_of_mut!(ECHO_RECEIVER_QUEUING) },
            unsafe { &mut *addr_of_mut!(ECHO_SENDER_SAMPLING) },
            unsafe { &mut *addr_of_mut!(ECHO_RECEIVER_SAMPLING) },
            &ECHO_ENTRY_FUNCTIONS,
        );
        <ApexLinuxPartition as ApexPartitionP4>::set_partition_mode(OperatingMode::Normal).unwrap();
    }

    fn warm_start(&self, ctx: &mut StartContext<ApexLinuxPartition>) {
        self.cold_start(ctx)
    }
}

fn main() {
    info!("Echo client main");

    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Info).unwrap();

    trace!("Echo client main: running partition");
    Echo.run()
}

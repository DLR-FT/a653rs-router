use a653rs::prelude::*;
use a653rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use echo::*;
use log::{info, trace, LevelFilter};

static mut ECHO_RECEIVER_SAMPLING: Option<SamplingPortDestination<ECHO_SIZE, ApexLinuxPartition>> =
    None;
static mut ECHO_SENDER_SAMPLING: Option<SamplingPortSource<ECHO_SIZE, ApexLinuxPartition>> = None;

// Not supported by a653rs-linux-hypervisor

// static ECHO_RECEIVER_QUEUING: OnceCell<QueuingPortDestination<ECHO_SIZE,
// ApexLinuxPartition>> =     OnceCell::new();
// static ECHO_SENDER_QUEUING: OnceCell<QueuingPortSource<ECHO_SIZE,
// Hypervisor>> = OnceCell::new();

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
    panic!("Not supported by a653rs-linux")
}

extern "C" fn client_receive_queuing() {
    panic!("Not supported by a653rs-linux")
}

extern "C" fn server_queuing() {
    panic!("Not supported by a653rs-linux")
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

impl Partition<ApexLinuxPartition> for Echo {
    fn cold_start(&self, ctx: &mut StartContext<ApexLinuxPartition>) {
        unsafe {
            cold_start_sampling(
                ctx,
                &mut ECHO_SENDER_SAMPLING,
                &mut ECHO_RECEIVER_SAMPLING,
                &ECHO_ENTRY_FUNCTIONS,
            );
        }
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

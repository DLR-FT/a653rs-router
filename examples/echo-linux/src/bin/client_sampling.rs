use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_linux::partition::ApexLinuxPartition as LinuxHypervisor;

use log::{info, trace};

use echo::LOG_LEVEL;

fn main() {
    info!("Echo client main");

    a653rs_linux::partition::ApexLogger::install_panic_hook();
    a653rs_linux::partition::ApexLogger::install_logger(LOG_LEVEL).unwrap();

    trace!("Echo client main: running partition");
    example::Partition.run();
}

// TODO: figure out how to include the other two hypervisors
#[partition(LinuxHypervisor)]
mod example {

    use super::*;

    // TODO: use ECHO_SIZE
    #[sampling_out(msg_size = "1kb")]
    struct EchoRequest;

    // TODO: use ECHO_SIZE
    #[sampling_in(refresh_period = "2000ms")]
    #[sampling_in(msg_size = "1kb")]
    struct EchoReply;

    #[start(warm)]
    fn warm_start(ctx: start::Context) {
        cold_start(ctx);
    }

    #[start(cold)]
    fn cold_start(mut ctx: start::Context) {
        // create the channels
        ctx.create_echo_request().unwrap();
        ctx.create_echo_reply().unwrap();

        // create the functions
        ctx.create_echo_receive().unwrap().start().unwrap();
        ctx.create_echo_send().unwrap().start().unwrap();
    }

    #[aperiodic(
        // There can be only one process with normal period
        // period: SystemTime::Infinite,
        // time_capacity: SystemTime::Infinite,
        time_capacity = "Infinite",
        // entry_point: self.entry_point_aperiodic,
        // stack_size: 20_000,
        stack_size = "20kb",
        // base_priority: 1,
        base_priority = 1,
        // deadline: Deadline::Soft,
        deadline = "Soft",
        // name: Name::from_str("EchoReceive").unwrap(),
        name = "EchoReceive",
    )]
    fn echo_receive(ctx: echo_receive::Context) {
        echo::run_echo_sampling_receiver(ctx.echo_reply.unwrap())
    }

    #[periodic(
        // period: SystemTime::Normal(Duration::from_secs(1)),
        period = "1s",
        // time_capacity: SystemTime::Infinite,
        time_capacity = "Infinite",
        // entry_point: self.entry_point_periodic,
        // stack_size: 20_000,
        stack_size = "20kb",
        // base_priority: 5,
        base_priority = 5,
        // deadline: Deadline::Soft,
        deadline = "Soft",
        // name: Name::from_str("EchoSend").unwrap(),
        // TODO: name is optional, keep it?
        name = "EchoSend",
    )]
    fn echo_send(ctx: echo_send::Context) {
        echo::run_echo_sampling_sender(ctx.echo_request.unwrap())
    }
}

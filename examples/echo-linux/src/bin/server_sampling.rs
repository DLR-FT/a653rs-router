use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_linux::partition::ApexLinuxPartition as LinuxHypervisor;

use log::info;

use echo::LOG_LEVEL;

fn main() {
    info!("Echo server main");

    a653rs_linux::partition::ApexLogger::install_panic_hook();
    a653rs_linux::partition::ApexLogger::install_logger(LOG_LEVEL).unwrap();

    example::Partition.run();
}

// TODO: figure out how to include the other two hypervisors
#[partition(LinuxHypervisor)]
mod example {

    use super::*;

    // Currently, the macro takes only string literals, so can't use a variable
    // ECHO_SIZE = 1kb
    #[sampling_out(msg_size = "1kb")]
    struct EchoReply;

    // ECHO_SIZE = 1kb
    #[sampling_in(refresh_period = "2000ms", msg_size = "1kb")]
    struct EchoRequest;

    #[start(warm)]
    fn warm_start(ctx: start::Context) {
        cold_start(ctx);
    }

    #[start(cold)]
    fn cold_start(mut ctx: start::Context) {
        info!("Echo server cold start");
        // create the channels
        ctx.create_echo_reply().unwrap();
        ctx.create_echo_request().unwrap();

        // create the functions
        ctx.create_server_main_loop().unwrap().start().unwrap();
    }

    #[aperiodic(
        // There can be only one process with normal period
        // period: SystemTime::Infinite,
        // time_capacity: SystemTime::Infinite,
        time_capacity = "Infinite",
        // entry_point: self.entry_point_aperiodic,
        // stack_size: 20_000,
        stack_size = "20KB",
        // base_priority: 5,
        base_priority = 5,
        // deadline: Deadline::Soft,
        deadline = "Soft",
        // name: Name::from_str("echo_server").unwrap(),
        name = "echo_server",
    )]
    // TODO: find sensible name
    fn server_main_loop(ctx: server_main_loop::Context) {
        echo::run_server_sampling_main(ctx.echo_reply.unwrap(), ctx.echo_request.unwrap())
    }
}

#![no_std]
use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_xng::apex::XngHypervisor;

use log::info;

use echo::LOG_LEVEL;

static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;
static TRACER: small_trace_gpio::GpioTracer = small_trace_gpio::GpioTracer::new();

#[no_mangle]
extern "C" fn main() {
    info!("Echo server main");

    TRACER.init();
    small_trace::set_tracer(&TRACER);
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(LOG_LEVEL);

    example::Partition.run();
}

// TODO: figure out how to include the other two hypervisors
#[partition(XngHypervisor)]
mod example {

    use super::*;

    // Currently, the macro takes only string literals, so can't use a variable
    // ECHO_SIZE = 1kb
    #[sampling_out(name = "Ch1", msg_size = "1kb")]
    struct EchoRequest;

    // ECHO_SIZE = 1kb
    #[sampling_in(refresh_period = "2000ms", msg_size = "1kb")]
    struct EchoReply;

    #[start(warm)]
    fn warm_start(ctx: start::Context) {
        cold_start(ctx);
    }

    #[start(cold)]
    fn cold_start(mut ctx: start::Context) {
        info!("Echo server cold start");
        // create the channels
        ctx.create_echo_request().unwrap();
        ctx.create_echo_reply().unwrap();

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
        stack_size = "20KiB",
        // base_priority: 5,
        base_priority = 5,
        // deadline: Deadline::Soft,
        deadline = "Soft",
        // name: Name::from_str("echo_server").unwrap(),
        name = "echo_server",
    )]
    // TODO: find sensible name
    fn server_main_loop(ctx: server_main_loop::Context) {
        echo::run_server_sampling_main(ctx.echo_request.unwrap(), ctx.echo_reply.unwrap())
    }
}

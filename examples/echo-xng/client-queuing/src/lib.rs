#![no_std]
use a653rs::partition;
use a653rs::prelude::PartitionExt;

use a653rs_xng::apex::XngHypervisor;

use echo::LOG_LEVEL;

use log::{info, trace};

static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;
static TRACER: small_trace_gpio::GpioTracer = small_trace_gpio::GpioTracer::new();

#[no_mangle]
extern "C" fn main() {
    info!("Echo client main");

    TRACER.init();
    small_trace::set_tracer(&TRACER);
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(LOG_LEVEL);

    trace!("Echo client main: running partition");
    example::Partition.run();
}

// TODO: figure out how to include the other two hypervisors
#[partition(XngHypervisor)]
mod example {

    use super::*;

    // TODO: use ECHO_SIZE
    #[queuing_out(msg_size = "1kb", discipline = "FIFO", msg_count = 10)]
    struct EchoRequest;

    // TODO: use ECHO_SIZE
    #[queuing_in(msg_size = "1kb", discipline = "FIFO", msg_count = 10)]
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
        // stack_size: 10000,
        stack_size = "10kb",
        // base_priority: 1,
        base_priority = 1,
        // deadline: Deadline::Soft,
        deadline = "Soft",
        // name: Name::from_str("EchoReceive").unwrap(),
        name = "EchoReceive",
    )]
    fn echo_receive(ctx: echo_receive::Context) {
        echo::run_echo_queuing_receiver(ctx.echo_reply.unwrap())
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
        echo::run_echo_queuing_sender(ctx.echo_request.unwrap())
    }
}

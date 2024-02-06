#![no_std]

use a653rs_router::router_partition;

static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;
static TRACER: small_trace_gpio::GpioTracer = small_trace_gpio::GpioTracer::new();
static LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

#[no_mangle]
pub extern "C" fn main() {
    TRACER.init();
    small_trace::set_tracer(&TRACER);
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(LOG_LEVEL);
    router::run()
}

#[router_partition(
    hypervisor = a653rs_xng::apex::XngHypervisor,
    interface(
        name = "NodeA",
        kind = a653rs_router_zynq7000::UartNetworkInterface,
        destination = "tx",
        mtu = "1.5KB",
        rate = "10MB",
        source = "rx"
    ),
    inputs = 1,
    outputs = 1,
    mtu = "2KB",
    port(queuing_in(
        name = "EchoReply",
        msg_size = "1KB",
        msg_count = "10",
        discipline = "Fifo"
    )),
    port(queuing_out(
        name = "EchoRequest",
        msg_size = "1KB",
        msg_count = "10",
        discipline = "Fifo"
    )),
    time_capacity = "50ms",
    stack_size = "30KB"
)]
mod router {}
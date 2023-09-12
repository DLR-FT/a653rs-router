#![no_std]

use router_partition_macros::router_partition;

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
    inputs = 1,
    outputs = 1,
    mtu = "2KB",
    time_capacity = "50ms",
    stack_size = "30KB",
    interface(
        name = "NodeB",
        kind = network_partition_uart::UartNetworkInterface,
        destination = "tx",
        mtu = "1.5KB",
        rate = "10MB",
        source = "rx"
    ),
    port(queuing_in(
        name = "EchoRequest",
        msg_size = "1KB",
        msg_count = "10",
        discipline = "Fifo"
    )),
    port(queuing_out(
        name = "EchoReply",
        msg_size = "1KB",
        msg_count = "10",
        discipline = "Fifo"
    ))
)]
mod router {}

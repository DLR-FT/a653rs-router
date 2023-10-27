use a653rs_router_partition_macros::router_partition;

static TRACER: a653rs_router_linux::LinuxTracer = a653rs_router_linux::LinuxTracer;
static LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

fn main() {
    small_trace::set_tracer(&TRACER);
    a653rs_linux::partition::ApexLogger::install_panic_hook();
    a653rs_linux::partition::ApexLogger::install_logger(LOG_LEVEL).unwrap();
    router::run()
}

#[router_partition(
    hypervisor = a653rs_linux::partition::ApexLinuxPartition,
    inputs = 1,
    outputs = 1,
    mtu = "2KB",
    time_capacity = "50ms",
    stack_size = "30KB",
    interface(
        name = "NodeB",
        kind = a653rs_router_linux::UdpNetworkInterface,
        destination = "192.168.1.2:8082",
        mtu = "1.5KB",
        rate = "10MB",
        source = "0.0.0.0:8081",
    ),
    port(sampling_in(name = "EchoRequest", msg_size = "1KB", refresh_period = "10s")),
    port(sampling_out(name = "EchoReply", msg_size = "1KB"))
)]
mod router {}

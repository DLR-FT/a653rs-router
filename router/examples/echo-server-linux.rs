use router_partition_macros::router_partition;

static TRACER: network_partition_linux::LinuxTracer = network_partition_linux::LinuxTracer;
static LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

fn main() {
    small_trace::set_tracer(&TRACER);
    a653rs_linux::partition::ApexLogger::install_panic_hook();
    a653rs_linux::partition::ApexLogger::install_logger(LOG_LEVEL).unwrap();
    router::run()
}

#[router_partition(
    hypervisor = a653rs_linux::partition::ApexLinuxPartition,
    interface(
        name = "NodeA",
        kind = network_partition_linux::UdpNetworkInterface,
        destination = "192.168.1.1:8081",
        mtu = "1.5KB",
        rate = "10MB",
        source = "0.0.0.0:8082",
    ),
    inputs = 1,
    outputs = 1,
    mtu = "2KB",
    port(sampling_out(name = "EchoRequest", msg_size = "1KB")),
    port(sampling_in(name = "EchoReply", msg_size = "1KB", refresh_period = "10s")),
    time_capacity = "50ms",
    stack_size = "30KB"
)]
mod router {}

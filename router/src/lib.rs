#![no_std]

#[cfg(any(
    all(feature = "dummy", any(feature = "linux", feature = "xng")),
    all(feature = "linux", any(feature = "xng", feature = "dummy")),
    all(feature = "xng", any(feature = "dummy", feature = "linux")),
))]
compile_error!("The features dummy, linux and xng are mutually exclusive, because they are meant for different platforms.");

#[cfg(all(feature = "echo", feature = "throughput"))]
compile_error!("The features `echo` and `throughput` are mutually exclusive, because they are meant for different configurations.");

#[cfg(all(
    any(feature = "echo", feature = "throughput"),
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
use a653rs::partition;

#[cfg(all(
    any(feature = "echo", feature = "throughput"),
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
use network_partition::router_config;

// =========== Set up logging and call entry function =========

#[cfg(feature = "xng")]
static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;

#[cfg(feature = "xng")]
static TRACER: small_trace_gpio::GpioTracer = small_trace_gpio::GpioTracer::new();

#[cfg(feature = "linux")]
static TRACER: network_partition_linux::LinuxTracer = network_partition_linux::LinuxTracer;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
static LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
pub fn run() {
    #[cfg(feature = "linux")]
    {
        small_trace::set_tracer(&TRACER);
        a653rs_linux::partition::ApexLogger::install_panic_hook();
        a653rs_linux::partition::ApexLogger::install_logger(LOG_LEVEL).unwrap();
    }

    #[cfg(feature = "xng")]
    {
        TRACER.init();
        small_trace::set_tracer(&TRACER);
        unsafe { log::set_logger_racy(&LOGGER).unwrap() };
        log::set_max_level(LOG_LEVEL);
    }

    #[cfg(any(feature = "echo", feature = "throughput"))]
    {
        log::debug!("Calling into partition");
        use a653rs::prelude::PartitionExt;
        router_partition::Partition.run()
    }
}

// TODO Use build.rs to generate the router configs. Nested cfg_attr will *not*
// work.

// ========================== Echo Router  ============================

#[cfg(all(feature = "echo", feature = "local"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 2, outputs = 2, mtu = "2KB")]
    struct Limits;
}

#[cfg(all(feature = "echo", feature = "xng", feature = "client"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
    struct Limits;

    #[interface(interface_type = network_partition_uart::UartNetworkInterface)]
    #[interface(source = "tx", destination = "rx")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct NodeB;
}

#[cfg(all(feature = "echo", feature = "xng", feature = "server"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
    struct Limits;

    #[interface(interface_type = network_partition_uart::UartNetworkInterface)]
    #[interface(source = "tx", destination = "rx")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct NodeA;
}

#[cfg(all(feature = "echo", feature = "linux", feature = "client"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
    struct Limits;

    #[interface(interface_type = network_partition_linux::UdpNetworkInterface)]
    #[interface(source = "0.0.0.0:8081", destination = "192.168.1.2:8082")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct NodeB;
}

#[cfg(all(feature = "echo", feature = "linux", feature = "server"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
    struct Limits;

    #[interface(interface_type = network_partition_linux::UdpNetworkInterface)]
    #[interface(source = "0.0.0.0:8082", destination = "192.168.1.1:8081")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct NodeA;
}

#[cfg(all(feature = "echo", feature = "dummy", feature = "client"))]
#[router_config(dummy_hypervisor::DummyScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
    struct Limits;

    #[interface(interface_type = dummy_hypervisor::DummyInterface)]
    #[interface(source = "client:8081", destination = "server:8082")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct Port1;
}

#[cfg(all(feature = "echo", feature = "dummy", feature = "server"))]
#[router_config(dummy_hypervisor::DummyScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
    struct Limits;

    #[interface(interface_type = dummy_hypervisor::DummyInterface)]
    #[interface(source = "server:8082", destination = "client:8081")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct Port1;
}

// ======================= Throughput Router ===================

#[cfg(all(feature = "throughput", feature = "xng", feature = "client"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 0, mtu = "100KB")]
    struct Limits;

    #[interface(interface_type = network_partition_uart::UartNetworkInterface)]
    #[interface(source = "tx", destination = "rx")]
    #[interface(rate = "10MB", mtu = "1KB")]
    struct NodeB;
}

#[cfg(all(feature = "throughput", feature = "xng", feature = "server"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 0, outputs = 1, mtu = "100KB")]
    struct Limits;

    #[interface(interface_type = network_partition_uart::UartNetworkInterface)]
    #[interface(source = "tx", destination = "rx")]
    #[interface(rate = "10MB", mtu = "1KB")]
    struct NodeA;
}

#[cfg(all(feature = "throughput", feature = "linux", feature = "client"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 0, mtu = "100KB")]
    struct Limits;

    #[interface(interface_type = network_partition_linux::UdpNetworkInterface)]
    #[interface(source = "client:8081", destination = "server:8082")]
    #[interface(rate = "10MB", mtu = "1KB")]
    struct NodeB;
}

#[cfg(all(feature = "throughput", feature = "linux", feature = "server"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 0, outputs = 1, mtu = "100KB")]
    struct Limits;

    #[interface(interface_type = network_partition_linux::UdpNetworkInterface)]
    #[interface(source = "server:8082", destination = "client:8081")]
    #[interface(rate = "10MB", mtu = "1KB")]
    struct NodeA;
}

#[cfg(all(feature = "throughput", feature = "dummy", feature = "client"))]
#[router_config(dummy_hypervisor::DummyScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 0, mtu = "100KB")]
    struct Limits;

    #[interface(interface_type = dummy_hypervisor::DummyInterface)]
    #[interface(source = "client:8081", destination = "server:8082")]
    #[interface(rate = "10MB", mtu = "1KB")]
    struct Port1;
}

#[cfg(all(feature = "throughput", feature = "dummy", feature = "server"))]
#[router_config(dummy_hypervisor::DummyScheduler)]
pub(crate) mod router {
    #[limits(inputs = 0, outputs = 1, mtu = "100KB")]
    struct Limits;

    #[interface(interface_type = dummy_hypervisor::DummyInterface)]
    #[interface(source = "server:8082", destination = "client:8081")]
    #[interface(rate = "10MB", mtu = "1KB")]
    struct Port1;
}

// ======================= Router for echo-local example ======================

#[cfg(all(feature = "local", feature = "echo", feature = "xng"))]
#[cfg_attr(feature = "xng", partition(a653rs_xng::apex::XngHypervisor))]
mod router_partition {

    #[queuing_in(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct EchoRequestCl;

    #[queuing_out(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct EchoRequestSrv;

    #[queuing_in(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct EchoReplySrv;

    #[queuing_out(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct EchoReplyCl;

    #[sampling_in(name = "RouterConfig", refresh_period = "10s", msg_size = "1KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        ctx.create_echo_request_cl().unwrap();
        ctx.create_echo_request_srv().unwrap();
        ctx.create_echo_reply_cl().unwrap();
        ctx.create_echo_reply_srv().unwrap();
        ctx.create_router_config().unwrap();
        ctx.create_aperiodic2().unwrap().start().unwrap();
    }

    #[aperiodic(
        name = "ap2",
        time_capacity = "50ms",
        stack_size = "30KB",
        base_priority = 5,
        deadline = "Soft"
    )]
    fn aperiodic2(ctx: aperiodic2::Context) {
        let router_config = ctx.router_config.unwrap();
        network_partition::run_router!(
            crate::router,
            Hypervisor {},
            router_config,
            [
                ("EchoRequestCl", ctx.echo_request_cl.unwrap()),
                ("EchoReplySrv", ctx.echo_reply_srv.unwrap()),
            ],
            [
                ("EchoRequestSrv", ctx.echo_request_srv.unwrap()),
                ("EchoReplyCl", ctx.echo_reply_cl.unwrap()),
            ]
        );
    }
}

// ======================= XNG Echo client ===========================

#[cfg(all(feature = "client", feature = "echo", feature = "xng"))]
#[cfg_attr(feature = "xng", partition(a653rs_xng::apex::XngHypervisor))]
mod router_partition {

    #[queuing_in(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct EchoRequest;

    #[queuing_out(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct EchoReply;

    #[sampling_in(name = "RouterConfig", refresh_period = "10s", msg_size = "1KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        ctx.create_echo_request().unwrap();
        ctx.create_echo_reply().unwrap();
        ctx.create_router_config().unwrap();
        ctx.create_aperiodic2().unwrap().start().unwrap();
    }

    #[aperiodic(
        name = "ap2",
        time_capacity = "50ms",
        stack_size = "30KB",
        base_priority = 5,
        deadline = "Soft"
    )]
    fn aperiodic2(ctx: aperiodic2::Context) {
        let router_config = ctx.router_config.unwrap();
        network_partition::run_router!(
            crate::router,
            Hypervisor {},
            router_config,
            [("EchoRequest", ctx.echo_request.unwrap())],
            [("EchoReply", ctx.echo_reply.unwrap())]
        );
    }
}

// ======================= XNG Echo server ===========================

#[cfg(all(feature = "server", feature = "echo", feature = "xng",))]
#[cfg_attr(feature = "xng", partition(a653rs_xng::apex::XngHypervisor))]
mod router_partition {

    #[queuing_out(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct EchoRequest;

    #[queuing_in(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct EchoReply;

    #[sampling_in(name = "RouterConfig", refresh_period = "10s", msg_size = "1KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        ctx.create_echo_request().unwrap();
        ctx.create_echo_reply().unwrap();
        ctx.create_router_config().unwrap();
        ctx.create_aperiodic2().unwrap().start().unwrap();
    }

    #[aperiodic(
        name = "ap2",
        time_capacity = "50ms",
        stack_size = "30KB",
        base_priority = 5,
        deadline = "Soft"
    )]
    fn aperiodic2(ctx: aperiodic2::Context) {
        let router_config = ctx.router_config.unwrap();
        network_partition::run_router!(
            crate::router,
            Hypervisor {},
            router_config,
            [("EchoReply", ctx.echo_reply.unwrap())],
            [("EchoRequest", ctx.echo_request.unwrap())]
        );
    }
}

// ======================= Linux Echo Partition Client
// ============================

#[cfg(all(feature = "client", feature = "echo", feature = "linux",))]
#[cfg_attr(
    feature = "linux",
    partition(a653rs_linux::partition::ApexLinuxPartition)
)]
mod router_partition {

    #[sampling_in(msg_size = "1KB", refresh_period = "10s")]
    struct EchoRequest;

    #[sampling_out(msg_size = "1KB")]
    struct EchoReply;

    #[sampling_in(name = "RouterConfig", refresh_period = "10s", msg_size = "1KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        ctx.create_echo_request().unwrap();
        ctx.create_echo_reply().unwrap();
        ctx.create_router_config().unwrap();
        ctx.create_aperiodic2().unwrap().start().unwrap();
    }

    #[aperiodic(
        name = "ap2",
        time_capacity = "50ms",
        stack_size = "30KB",
        base_priority = 5,
        deadline = "Soft"
    )]
    fn aperiodic2(ctx: aperiodic2::Context) {
        let router_config = ctx.router_config.unwrap();
        network_partition::run_router!(
            crate::router,
            Hypervisor {},
            router_config,
            [("EchoRequest", ctx.echo_request.unwrap())],
            [("EchoReply", ctx.echo_reply.unwrap())]
        );
    }
}

// ======================= Linux Echo server ===========================

#[cfg(all(feature = "server", feature = "echo", feature = "linux",))]
#[cfg_attr(
    feature = "linux",
    partition(a653rs_linux::partition::ApexLinuxPartition)
)]
mod router_partition {

    #[sampling_out(msg_size = "1KB")]
    struct EchoRequest;

    #[sampling_in(msg_size = "1KB", refresh_period = "10s")]
    struct EchoReply;

    #[sampling_in(name = "RouterConfig", refresh_period = "10s", msg_size = "1KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        ctx.create_echo_request().unwrap();
        ctx.create_echo_reply().unwrap();
        ctx.create_router_config().unwrap();
        ctx.create_aperiodic2().unwrap().start().unwrap();
    }

    #[aperiodic(
        name = "ap2",
        time_capacity = "50ms",
        stack_size = "30KB",
        base_priority = 5,
        deadline = "Soft"
    )]
    fn aperiodic2(ctx: aperiodic2::Context) {
        let router_config = ctx.router_config.unwrap();
        network_partition::run_router!(
            crate::router,
            Hypervisor {},
            router_config,
            [("EchoReply", ctx.echo_reply.unwrap())],
            [("EchoRequest", ctx.echo_request.unwrap())]
        );
    }
}

// ======================= Throughput partition ============================

#[cfg(all(
    feature = "throughput",
    any(feature = "dummy", feature = "linux", feature = "xng")
))]
#[cfg_attr(feature = "dummy", partition(dummy_hypervisor::DummyHypervisor))]
#[cfg_attr(
    feature = "linux",
    partition(a653rs_linux::partition::ApexLinuxPartition)
)]
#[cfg_attr(feature = "xng", partition(a653rs_xng::apex::XngHypervisor))]
mod router_partition {
    // Only one of the ports will actually be used for each server and client, but
    // this way we can also use the partition for non-networked tests.

    // TODO use proper config for more throughput
    #[queuing_in(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct ThroughputIn;

    #[queuing_out(msg_size = "1KB", msg_count = "10", discipline = "Fifo")]
    struct ThroughputOut;

    #[sampling_in(name = "RouterConfig", refresh_period = "1s", msg_size = "1KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        #[cfg(feature = "client")]
        ctx.create_throughput_in().unwrap();

        #[cfg(feature = "server")]
        ctx.create_throughput_out().unwrap();

        ctx.create_router_config().unwrap();
        ctx.create_aperiodic2().unwrap().start().unwrap();
    }

    #[aperiodic(
        name = "ap2",
        time_capacity = "50ms",
        stack_size = "30KB",
        base_priority = 5,
        deadline = "Soft"
    )]
    fn aperiodic2(ctx: aperiodic2::Context) {
        let router_config = ctx.router_config.unwrap();

        #[cfg(feature = "client")]
        network_partition::run_router!(
            crate::router,
            Hypervisor {},
            router_config,
            [("ThroughputIn", ctx.throughput_in.unwrap())],
            []
        );

        #[cfg(feature = "server")]
        network_partition::run_router!(
            crate::router,
            Hypervisor {},
            router_config,
            [],
            [("ThroughputOut", ctx.throughput_out.unwrap())]
        );
    }
}

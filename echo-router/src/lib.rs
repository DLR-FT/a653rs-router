#![cfg_attr(
    all(any(features = "xng", features = "dummy"), not(features = "linux")),
    no_std
)]
#![cfg_attr(feature = "dummy", allow(dead_code))]

#[cfg(any(
    all(feature = "dummy", any(feature = "linux", feature = "xng")),
    all(feature = "linux", any(feature = "xng", feature = "dummy")),
    all(feature = "xng", any(feature = "dummy", feature = "linux")),
))]
compile_error!("The features dummy, linux and xng are mutually exclusive, because they are meant for different platforms.");

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
use a653rs::partition;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
use a653rs::prelude::PartitionExt;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
use network_partition::router_config;

// =========== Set up logging and call entry function =========

#[cfg(feature = "xng")]
static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;

#[cfg(feature = "linux")]
static LOGGER: a653rs_linux::partition::ApexLogger = a653rs_linux::partition::ApexLogger();

#[cfg(feature = "xng")]
static TRACER: small_trace_gpio::GpioTracer = small_trace_gpio::GpioTracer::new();

#[cfg(feature = "linux")]
static TRACER: network_partition_linux::LinuxTracer = network_partition_linux::LinuxTracer;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
static LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
pub fn run() {
    #[cfg(feature = "xng")]
    {
        TRACER.init();
    }

    #[cfg(any(feature = "xng", feature = "linux"))]
    {
        small_trace::set_tracer(&TRACER);
        unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    }
    log::set_max_level(LOG_LEVEL);
    router_partition::Partition.run()
}

// ========================== Router  ============================

// #[cfg_attr(all(feature = "dummy", feature = "client"),
// router_config(dummy_hypervisor::DummyScheduler))]

#[cfg(all(feature = "xng", feature = "client"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler<2>)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "1Kib")]
    struct Limits;

    #[interface(interface_type = network_partition_uart::UartNetworkInterface)]
    #[interface(source = "tx", destination = "rx")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct Port1;
}

#[cfg(all(feature = "xng", feature = "server"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler<2>)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "1Kib")]
    struct Limits;

    #[interface(interface_type = network_partition_uart::UartNetworkInterface)]
    #[interface(source = "tx", destination = "rx")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct Port1;
}

#[cfg(all(feature = "linux", feature = "client"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler<2>)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "1Kib")]
    struct Limits;

    #[interface(interface_type = network_partition_linux::UdpNetworkInterface)]
    #[interface(source = "127.0.0.1:8081", destination = "127.0.0.1:8082")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct Port1;
}

#[cfg(all(feature = "linux", feature = "server"))]
#[router_config(network_partition::prelude::DeadlineRrScheduler<2>)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "1Kib")]
    struct Limits;

    #[interface(interface_type = network_partition_linux::UdpNetworkInterface)]
    #[interface(source = "127.0.0.1:8082", destination = "127.0.0.1:8081")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct Port1;
}

#[cfg(all(feature = "dummy", feature = "client"))]
#[router_config(dummy_hypervisor::DummyScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "1Kib")]
    struct Limits;

    #[interface(interface_type = dummy_hypervisor::DummyInterface)]
    #[interface(source = "127.0.0.1:8081", destination = "127.0.0.1:8082")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct Port1;
}

#[cfg(all(feature = "dummy", feature = "server"))]
#[router_config(dummy_hypervisor::DummyScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "1Kib")]
    struct Limits;

    #[interface(interface_type = dummy_hypervisor::DummyInterface)]
    #[interface(source = "127.0.0.1:8082", destination = "127.0.0.1:8081")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct Port1;
}

// ======================= Partition ============================

#[cfg(any(feature = "dummy", feature = "linux", feature = "xng"))]
#[cfg_attr(feature = "dummy", partition(dummy_hypervisor::DummyHypervisor))]
#[cfg_attr(
    feature = "linux",
    partition(a653rs_linux::partition::ApexLinuxPartition)
)]
#[cfg_attr(feature = "xng", partition(a653rs_xng::apex::XngHypervisor))]
mod router_partition {

    #[sampling_in(msg_size = "10KB", refresh_period = "10s")]
    struct EchoRequest;

    #[sampling_out(msg_size = "10KB")]
    struct EchoReply;

    #[sampling_in(name = "RouterConfig", refresh_period = "1s", msg_size = "10KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        ctx.create_aperiodic2().unwrap();
        ctx.create_echo_request().unwrap();
        ctx.create_echo_reply().unwrap();
        ctx.create_router_config().unwrap();
    }

    #[aperiodic(
        name = "ap2",
        time_capacity = "500ms",
        stack_size = "16MB",
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

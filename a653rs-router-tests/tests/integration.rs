use a653rs::partition;
use a653rs_router::router_config;

#[router_config(a653rs_router::prelude::DeadlineRrScheduler)]
pub(crate) mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
    struct Limits;
    #[interface(interface_type = a653rs_router_linux::UdpNetworkInterface)]
    #[interface(source = "tx", destination = "rx")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct NodeB;
}

#[partition(a653rs_linux::partition::ApexLinuxPartition)]
mod router_partition {
    use a653rs_router::run_router;

    #[sampling_in(msg_size = "1KB", refresh_period = "1s")]
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
        let time_source = a653rs_linux::partition::ApexLinuxPartition {};
        let router_config = ctx.router_config.unwrap();
        let echo_request = ctx.echo_request.unwrap();
        let echo_reply = ctx.echo_reply.unwrap();
        run_router!(
            crate::router,
            time_source,
            router_config,
            [("EchoRequest", echo_request)],
            [("EchoReply", echo_reply)]
        );
    }
}

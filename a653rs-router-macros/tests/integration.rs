use a653rs::partition;
use a653rs_router_macros::router_config;

#[router_config(a653rs_router::prelude::DeadlineRrScheduler)]
mod example_router {
    #[limits(inputs = 1, outputs = 1, mtu = "10KB")]
    struct Limits;

    #[interface(
        interface_type = a653rs_router_linux::UdpNetworkInterface,
        rate = "10MB",
        mtu = "1.5KB",
        source = "127.0.0.1:8081",
        destination = "127.0.0.1:8082"
    )]
    struct Dummy8081;
}

#[partition(a653rs_linux::partition::ApexLinuxPartition)]
mod example {
    #[sampling_in(msg_size = "1KB", refresh_period = "3s")]
    struct Channel1;

    #[sampling_out(msg_size = "500B")]
    struct Channel2;

    #[sampling_in(name = "RouterConfig", refresh_period = "1s", msg_size = "1KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        ctx.create_aperiodic2().unwrap().start().unwrap();
        ctx.create_channel_1().unwrap();
        ctx.create_router_config().unwrap();
    }

    #[aperiodic(
        name = "ap2",
        time_capacity = "200ms",
        stack_size = "16MB",
        base_priority = 5,
        deadline = "Soft"
    )]
    fn aperiodic2(ctx: aperiodic2::Context) {
        let router_config = ctx.router_config.unwrap();
        a653rs_router_macros::run_router!(
            crate::example_router,
            a653rs_linux::partition::ApexLinuxPartition,
            router_config,
            [("Ch1", ctx.channel_1.unwrap())],
            [("Ch2", ctx.channel_2.unwrap())]
        );
    }
}

use a653rs_router::router_config;

#[router_config(a653rs_router::prelude::DeadlineRrScheduler)]
pub mod router {
    #[limits(inputs = 1, outputs = 1, mtu = "2KB")]
    struct Limits;

    #[interface(interface_type = a653rs_router_linux::UdpNetworkInterface)]
    #[interface(source = "tx", destination = "rx")]
    #[interface(rate = "10MB", mtu = "1.5KB")]
    struct NodeB;
}

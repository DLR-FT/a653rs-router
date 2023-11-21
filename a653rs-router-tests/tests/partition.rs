use a653rs::router_partition;

#[router_partition(
     hypervisor = a653rs_linux::partition::ApexLinuxPartition,
     interface(
         name = "51234",
         kind = a653rs_router_linux::UdpNetworkInterface,
         destination = "127.0.0.1:51234",
         mtu = "1KB",
         rate = "100MB",
         source = "127.0.0.1:54234"
     ),
     inputs = 1,
     outputs = 1,
     mtu = "1.5KB",
     port(sampling_in(name = "[IGS]", msg_size = "1KB", refresh_period = "10s")),
     port(sampling_out(name = "CAS", msg_size = "1KB")),
     stack_size = "50MB",
     time_capacity = "5ms"
 )]
mod my_router {}

fn partition_entry_function() {
    my_router::run()
}

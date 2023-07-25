#![no_std]

use a653rs::partition;
use a653rs::prelude::PartitionExt;

pub fn run() {
    configurator::Partition.run();
}

#[partition(a653rs_linux::partition::ApexLinuxPartition)]
mod configurator {
    use a653rs_postcard::sampling::SamplingPortSourceExt;
    use core::time::Duration;
    use log::error;
    use log::*;
    use network_partition::prelude::Config;
    use network_partition::prelude::*;

    #[allow(dead_code)]
    const LOG_LEVEL: LevelFilter = LevelFilter::Info;

    #[sampling_out(name = "RouterConfig", msg_size = "1KB")]
    struct RouterConfig;

    #[start(cold)]
    fn cold_start(ctx: start::Context) {
        warm_start(ctx);
    }

    #[start(warm)]
    fn warm_start(mut ctx: start::Context) {
        ctx.create_router_config().unwrap();
        ctx.create_periodic().unwrap().start().unwrap();
    }

    fn config(cfg: ConfigOption) -> ConfigResult<2, 2> {
        match cfg {
            ConfigOption::EchoClient => Config::builder()
                .virtual_link(1, "EchoRequest")?
                .destination(1, "Udp8081")?
                .schedule(1, Duration::from_millis(10))?
                .virtual_link(2, "Udp8081")?
                .destination(2, "EchoReply")?
                .schedule(2, Duration::from_millis(10))?
                .build(),
            ConfigOption::EchoServer => Config::builder()
                .virtual_link(1, "Udp8082")?
                .destination(1, "EchoRequest")?
                .schedule(1, Duration::from_millis(10))?
                .virtual_link(2, "EchoReply")?
                .destination(2, "Udp8082")?
                .schedule(2, Duration::from_millis(10))?
                .build(),
            ConfigOption::Default => Ok(Config::default()),
        }
    }

    #[allow(dead_code)]
    enum ConfigOption {
        EchoClient,
        EchoServer,
        Default,
    }

    #[cfg(feature = "client")]
    const CONFIG: ConfigOption = ConfigOption::EchoClient;

    #[cfg(feature = "server")]
    const CONFIG: ConfigOption = ConfigOption::EchoServer;

    #[cfg(not(any(feature = "server", feature = "client")))]
    const CONFIG: ConfigOption = ConfigOption::Default;

    #[periodic(
        period = "1s",
        time_capacity = "Infinite",
        stack_size = "100KB",
        base_priority = 1,
        deadline = "Hard"
    )]
    fn periodic(ctx: periodic::Context) {
        a653rs_linux::partition::ApexLogger::install_panic_hook();
        a653rs_linux::partition::ApexLogger::install_logger(LOG_LEVEL).unwrap();
        info!("Running configurator");
        let port = ctx.router_config.unwrap();
        loop {
            let cfg = config(CONFIG).unwrap();
            info!("Sending configuration: {cfg:?}");
            if let Err(e) = port.send_type(cfg) {
                error!("Failed to update config: {e:?}");
            }
            ctx.periodic_wait().unwrap();
        }
    }
}

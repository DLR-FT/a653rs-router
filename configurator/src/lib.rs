#![no_std]

#[cfg(all(feature = "linux", feature = "xng"))]
compile_error!("The features `linux` or `xng` are mutually exclusive.");

#[cfg(any(feature = "linux", feature = "xng"))]
use a653rs::partition;
#[cfg(any(feature = "linux", feature = "xng"))]
use a653rs::prelude::PartitionExt;

#[allow(dead_code)]
const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;

#[cfg(feature = "xng")]
static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;

#[cfg(any(feature = "linux", feature = "xng"))]
pub fn run() {
    use log::*;

    #[cfg(feature = "linux")]
    {
        a653rs_linux::partition::ApexLogger::install_panic_hook();
        a653rs_linux::partition::ApexLogger::install_logger(LOG_LEVEL).unwrap();
    }
    #[cfg(feature = "xng")]
    {
        unsafe { log::set_logger_racy(&LOGGER).unwrap() };
        log::set_max_level(LOG_LEVEL);
    }
    info!("Running configurator");
    configurator::Partition.run();
}

#[cfg(any(feature = "linux", feature = "xng"))]
#[cfg_attr(
    feature = "linux",
    partition(a653rs_linux::partition::ApexLinuxPartition)
)]
#[cfg_attr(feature = "xng", partition(a653rs_xng::apex::XngHypervisor))]
mod configurator {
    use a653rs_postcard::sampling::SamplingPortSourceExt;
    use core::time::Duration;
    use log::*;
    use network_partition::prelude::Config;
    use network_partition::prelude::*;

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
                .destination(1, "NodeB")?
                .schedule(1, Duration::from_millis(10))?
                .virtual_link(2, "NodeB")?
                .destination(2, "EchoReply")?
                .schedule(2, Duration::from_millis(10))?
                .build(),
            ConfigOption::EchoServer => Config::builder()
                .virtual_link(1, "NodeA")?
                .destination(1, "EchoRequest")?
                .schedule(1, Duration::from_millis(10))?
                .virtual_link(2, "EchoReply")?
                .destination(2, "NodeA")?
                .schedule(2, Duration::from_millis(10))?
                .build(),
            ConfigOption::EchoLocal => Config::builder()
                .virtual_link(1, "EchoRequestCl")?
                .destination(1, "EchoRequestSrv")?
                .schedule(1, Duration::from_millis(5))?
                .virtual_link(2, "EchoReplySrv")?
                .destination(2, "EchoReplyCl")?
                .schedule(2, Duration::from_millis(5))?
                .build(),
            ConfigOption::Default => Ok(Config::default()),
        }
    }

    #[allow(dead_code)]
    enum ConfigOption {
        EchoClient,
        EchoServer,
        EchoLocal,
        Default,
    }

    #[cfg(feature = "client")]
    const CONFIG: ConfigOption = ConfigOption::EchoClient;

    #[cfg(feature = "server")]
    const CONFIG: ConfigOption = ConfigOption::EchoServer;

    #[cfg(feature = "local")]
    const CONFIG: ConfigOption = ConfigOption::EchoLocal;

    #[cfg(not(any(feature = "local", feature = "server", feature = "client")))]
    const CONFIG: ConfigOption = ConfigOption::Default;

    #[periodic(
        period = "1s",
        time_capacity = "Infinite",
        stack_size = "20KB",
        base_priority = 5,
        deadline = "Soft"
    )]
    fn periodic(ctx: periodic::Context) {
        debug!("Running configurator periodic process");
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

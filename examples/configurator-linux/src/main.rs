use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_linux::partition::ApexLogger;
use log::*;

const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

pub fn main() {
    run();
}

pub fn run() -> ! {
    ApexLogger::install_logger(LOG_LEVEL).unwrap();
    ApexLogger::install_panic_hook();

    info!("Running configurator");
    configurator::Partition.run()
}

#[partition(a653rs_linux::partition::ApexLinuxPartition)]
mod configurator {
    use log::*;

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

        configurator::configure(port);
    }
}

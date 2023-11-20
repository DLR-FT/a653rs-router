#![no_std]

use a653rs::partition;
use a653rs::prelude::PartitionExt;
use log::*;

const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

#[cfg(not(test))]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn main() {
    run();
}

pub fn run() -> ! {
    static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;

    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(LOG_LEVEL);

    info!("Running configurator");
    configurator::Partition.run()
}

#[partition(a653rs_xng::apex::XngHypervisor)]
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

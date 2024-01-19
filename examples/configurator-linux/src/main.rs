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
    use a653rs_router::prelude::Config;
    use log::*;
    use signal_hook::consts::SIGHUP;
    use std::{
        fs::File,
        io::{BufReader, ErrorKind},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    const INPUTS: usize = 2;
    const OUTPUTS: usize = 2;

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
        info!("Running configurator periodic process");
        let port = ctx.router_config.unwrap();

        let reload = Arc::new(AtomicBool::new(true));
        signal_hook::flag::register(SIGHUP, Arc::clone(&reload))
            .expect("Failed to set up signal handler");

        loop {
            if reload.load(Ordering::Relaxed) {
                if let Err(e) = load_config("/route-table.json", port) {
                    error!("Failed to load config: {:?}", e)
                } else {
                    reload.store(false, Ordering::Relaxed);
                }
            }
            <Hypervisor as ApexTimeP4Ext>::periodic_wait().unwrap();
        }
    }

    fn load_config(
        cfg_path: &str,
        port: &SamplingPortSource<1000, Hypervisor>,
    ) -> Result<(), std::io::Error> {
        let cfg: Config<INPUTS, OUTPUTS> = {
            let cfg = BufReader::new(File::open(cfg_path)?);
            serde_json::from_reader(cfg)?
        };
        let mut buf = [0u8; 1000];
        info!("Loading configuration: {:?}", cfg.clone());
        let buf = postcard::to_slice::<Config<INPUTS, OUTPUTS>>(&cfg, &mut buf).map_err(|e| {
            std::io::Error::new(
                ErrorKind::Other,
                format!("Failed to serialize config {e:?}"),
            )
        })?;
        port.send(buf).map_err(|e| {
            std::io::Error::new(
                ErrorKind::Other,
                format!("Failed to send new config: {e:?}"),
            )
        })
    }
}

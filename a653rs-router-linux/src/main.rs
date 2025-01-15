use a653rs::bindings::ApexPartitionP4;
use a653rs::prelude::{ApexTimeP4Ext, Name, OperatingMode, Partition, PartitionExt, StartContext};
use a653rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use a653rs_router::prelude::{RouterConfig, RouterState, VirtualLinksConfig};
use a653rs_router_linux::*;
use core::str::FromStr;
use std::{fs::File, io::BufReader};

const MTU: usize = 2_000;
const INPUTS: usize = 8;
const OUTPUTS: usize = 8;
const INTERFACES: usize = 8;
const PORTS: usize = 8;
const NAME: &str = "Router";
const CONFIG_PATH: &str = "/router.yml";

type NetIntf = UdpNetworkInterface<MTU>;

static mut ROUTER: Option<RouterState<ApexLinuxPartition, NetIntf, INTERFACES, PORTS>> = None;
static mut VL_CFG: Option<VirtualLinksConfig<INPUTS, OUTPUTS>> = None;

#[derive(Debug)]
struct RouterPartition;

type Hypervisor = ApexLinuxPartition;

impl Partition<Hypervisor> for RouterPartition {
    fn cold_start(&self, ctx: &mut StartContext<Hypervisor>) {
        let reader = BufReader::new(File::open(CONFIG_PATH).unwrap());
        let cfg: RouterConfig<INPUTS, OUTPUTS, INTERFACES, PORTS> =
            serde_yaml::from_reader(reader).unwrap();
        _ = unsafe { VL_CFG.insert(cfg.virtual_links) };

        let router = RouterState::create::<NetIntf>(
            ctx,
            Name::from_str(NAME).unwrap(),
            cfg.interfaces,
            cfg.ports,
            cfg.period,
            cfg.time_capacity,
            cfg.stack_size,
            entry_point,
        )
        .unwrap();
        _ = unsafe { ROUTER.insert(router) };
        let router = unsafe { ROUTER.as_ref() }.unwrap();
        router.start().unwrap();
        <ApexLinuxPartition as ApexPartitionP4>::set_partition_mode(OperatingMode::Normal).unwrap();
    }

    fn warm_start(&self, ctx: &mut StartContext<Hypervisor>) {
        self.cold_start(ctx)
    }
}

extern "C" fn entry_point() {
    let router = unsafe { ROUTER.as_ref() }.unwrap();
    let cfg = unsafe { VL_CFG.as_ref() }.unwrap().clone();
    let mut state = router
        .router::<INPUTS, OUTPUTS, MTU>(cfg, &Hypervisor::get_time().unwrap_duration())
        .unwrap();
    loop {
        let res = state.forward::<MTU, _>(&ApexLinuxPartition);
        #[cfg(feature = "log")]
        {
            use a653rs_router::prelude::Error;
            use log::{debug, trace};

            match res {
                Ok(Some(v)) => {
                    debug!("Forwarded VL {}", v);
                    continue;
                }
                Ok(None) => trace!("Scheduled no VL"),
                Err(Error::Port(e)) => {
                    trace!("Port send/receive failed temporarily: {}", e);
                    continue;
                }
                Err(e) => {
                    debug!("Failed to forward message: {}", e);
                    continue;
                }
            }
        }
        #[cfg(not(feature = "log"))]
        let _res = res;
        Hypervisor::periodic_wait().unwrap();
    }
}

fn main() {
    ApexLogger::install_panic_hook();
    #[cfg(feature = "log")]
    {
        ApexLogger::install_logger(log::LevelFilter::Debug).unwrap();
    }
    RouterPartition.run()
}

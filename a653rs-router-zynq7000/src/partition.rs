#![no_std]

use a653rs::bindings::{ApexPartitionP4, OperatingMode};
use a653rs::prelude::{Name, Partition, PartitionExt, StartContext};
use a653rs_router::prelude::{RouterConfig, RouterState, VirtualLinksConfig};
use a653rs_router_zynq7000::UartNetworkInterface;
use a653rs_xng::apex::XngHypervisor;
use core::str::FromStr;
use log::*;

#[cfg(feature = "log")]
use xng_rs_log::XalLogger;

const MTU: usize = 2_000;
const INPUTS: usize = 8;
const OUTPUTS: usize = 8;
const INTERFACES: usize = 8;
const PORTS: usize = 8;
const NAME: &str = "Router";
const CONFIG_MEMORY_AREA: usize = 0x16000000;
const CONFIG_MEMORY_AREA_SIZE: usize = 10_000;

type NetIntf = UartNetworkInterface<MTU>;

#[cfg(feature = "log")]
static LOGGER: XalLogger = XalLogger;

static mut ROUTER: Option<RouterState<XngHypervisor, NetIntf, INTERFACES, PORTS>> = None;
static mut VL_CFG: Option<VirtualLinksConfig<INPUTS, OUTPUTS>> = None;

#[derive(Debug)]
struct RouterPartition;

impl Partition<XngHypervisor> for RouterPartition {
    fn cold_start(&self, ctx: &mut StartContext<XngHypervisor>) {
        info!("Running router cold_start");
        let cfg = unsafe {
            core::slice::from_raw_parts(CONFIG_MEMORY_AREA as *const u8, CONFIG_MEMORY_AREA_SIZE)
        };
        let cfg: RouterConfig<INPUTS, OUTPUTS, INTERFACES, PORTS> =
            postcard::from_bytes(&cfg).expect("Failed to read configuration");
        info!("Have router configuration {:?}", cfg);
        _ = unsafe { VL_CFG.insert(cfg.virtual_links) };
        let router = RouterState::create::<NetIntf>(
            ctx,
            Name::from_str(NAME).unwrap(),
            cfg.interfaces,
            cfg.ports,
            cfg.stack_size,
            entry_point,
        )
        .expect("Failed to init router state");
        _ = unsafe { ROUTER.insert(router) };
        let router = unsafe { ROUTER.as_ref() }.unwrap();
        info!("Starting router process");
        router.start().expect("Failed to start process");
        info!("Started router process");
        <XngHypervisor as ApexPartitionP4>::set_partition_mode(OperatingMode::Normal).unwrap();
    }

    fn warm_start(&self, ctx: &mut StartContext<XngHypervisor>) {
        self.cold_start(ctx)
    }
}

extern "C" fn entry_point() {
    use a653rs::prelude::ApexTimeP4Ext;

    info!("Running router entry_point");
    let router = unsafe { ROUTER.as_ref() }.unwrap();
    let cfg = unsafe { VL_CFG.as_ref() }.unwrap().clone();
    let mut router = router
        .router::<INPUTS, OUTPUTS, MTU>(cfg, &XngHypervisor::get_time().unwrap_duration())
        .unwrap();
    loop {
        let res = router.forward::<MTU, _>(&XngHypervisor);
        #[cfg(feature = "log")]
        {
            use a653rs_router::prelude::Error;
            use log::*;
            match res {
                Ok(Some(v)) => debug!("Forwarded VL {}", v),
                Ok(None) => trace!("Scheduled no VL"),
                Err(Error::Port(e)) => trace!("Port send/receive failed temporarily: {}", e),
                Err(e) => debug!("Failed to forward message on VL: {}", e),
            }
        }

        #[cfg(not(feature = "log"))]
        {
            let _res = res;
        }
    }
}

#[no_mangle]
pub extern "C" fn main() {
    #[cfg(feature = "log")]
    {
        unsafe { set_logger_racy(&LOGGER).unwrap() };
        set_max_level(log::LevelFilter::Info);
    }
    info!("Running router main");
    RouterPartition.run()
}

#[cfg(not(feature = "log"))]
#[panic_handler]
fn my_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

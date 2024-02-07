#[cfg(feature = "serde")]
use log::{debug, error, info, trace, warn};

#[cfg(feature = "serde")]
use crate::scheduler::TimeSource;

#[cfg(feature = "serde")]
use crate::{
    config::Config,
    error::Error,
    reconfigure::{Configurator, Resources},
    router::RouterInput,
    scheduler::Scheduler,
};

#[cfg(feature = "serde")]
use crate::prelude::Router;

/// Runs the router.
#[cfg(feature = "serde")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "serde")))]
pub fn run<const IN: usize, const OUT: usize, const BUF_LEN: usize>(
    time_source: &'_ dyn TimeSource,
    router_config: &'_ dyn RouterInput,
    resources: Resources<IN, OUT>,
    scheduler: &'_ mut dyn Scheduler,
) -> ! {
    info!("Running a653rs-router");
    let mut cfg: Config<IN, OUT> = Config::default();
    let mut router: Option<Router<IN, OUT>> = None;
    let mut reconfigure_timer: u32 = 0;
    loop {
        if reconfigure_timer == 0 {
            let new_config = Configurator::fetch_config(router_config);
            match new_config {
                Ok(new_cfg) => {
                    if new_cfg != cfg {
                        debug!("New config = {new_cfg:?}");
                        match Configurator::reconfigure(&resources, scheduler, &new_cfg) {
                            Ok(r) => {
                                router = Some(r);
                                cfg = new_cfg;
                                info!("Reconfigured");
                            }
                            Err(e) => warn!("Failed to reconfigure: {e}"),
                        }
                    }
                }
                Err(e) => debug!("Fetching configuration failed: {e}"),
            }
        }
        reconfigure_timer = (reconfigure_timer + 1) % 0x10000;
        if let Some(ref router) = router {
            let res = router.forward::<BUF_LEN>(scheduler, time_source);
            match res {
                Ok(Some(v)) => trace!("Forwarded VL {v}"),
                Ok(None) => trace!("Scheduled no VL"),
                Err(Error::Port(e)) => debug!("Port send/receive failed temporarily: {e}"),
                Err(e) => error!("Failed to forward message on VL: {e}"),
            }
        }
    }
}

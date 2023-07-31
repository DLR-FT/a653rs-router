#[cfg(feature = "serde")]
use log::{debug, error, info, trace, warn};

#[cfg(feature = "serde")]
use crate::scheduler::TimeSource;

#[cfg(feature = "serde")]
use crate::{
    config::Config,
    reconfigure::{Configurator, Resources},
    router::RouterInput,
    scheduler::Scheduler,
};

/// Runs the router
/// TODO make config from port optional
#[cfg(feature = "serde")]
pub fn run<const IN: usize, const OUT: usize, const BUF_LEN: usize>(
    time_source: &'_ dyn TimeSource,
    router_config: &'_ dyn RouterInput,
    resources: Resources<IN, OUT>,
    scheduler: &'_ mut dyn Scheduler,
) -> ! {
    use crate::prelude::Router;

    info!("Running network-partition");
    let mut cfg: Config<IN, OUT> = Config::default();
    let mut router: Option<Router<IN, OUT>> = None;
    loop {
        match Configurator::fetch_config(router_config) {
            Ok(new_cfg) => {
                if new_cfg != cfg {
                    debug!("New config = {new_cfg:?}");
                    match Configurator::reconfigure(&resources, scheduler, &new_cfg) {
                        Ok(r) => {
                            router = Some(r);
                            cfg = new_cfg;
                            info!("Reconfigured");
                        }
                        Err(e) => warn!("Failed to reconfigure: {e:?}"),
                    }
                }
            }
            Err(e) => error!("Failed to fetch config: {e:?}"),
        }
        if let Some(ref router) = router {
            match router.forward::<BUF_LEN>(scheduler, time_source) {
                Ok(Some(v)) => debug!("Forwarded VL {v}"),
                Ok(None) => trace!("Scheduled no VL"),
                Err(e) => error!("Failed to forward VL: {e:?}"),
            }
        }
    }
}

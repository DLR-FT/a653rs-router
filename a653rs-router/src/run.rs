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
    router_trace!("Running a653rs-router");
    let mut cfg: Config<IN, OUT> = Config::default();
    let mut router: Option<Router<IN, OUT>> = None;
    let mut reconfigure_timer: u32 = 0;
    loop {
        if reconfigure_timer == 0 {
            let new_config = Configurator::fetch_config(router_config);
            match new_config {
                Ok(new_cfg) => {
                    if new_cfg != cfg {
                        router_trace!("New config = {new_cfg:?}");
                        match Configurator::reconfigure(&resources, scheduler, &new_cfg) {
                            Ok(r) => {
                                router = Some(r);
                                cfg = new_cfg;
                                router_trace!("Reconfigured");
                            }
                            Err(e) => router_debug!("Failed to reconfigure: {}", e),
                        }
                    }
                }
                Err(e) => router_debug!("Fetching configuration failed: {}", e),
            }
        }
        reconfigure_timer = (reconfigure_timer + 1) % 0x10000;
        if let Some(ref router) = router {
            let res = router.forward::<BUF_LEN>(scheduler, time_source);
            match res {
                Ok(Some(v)) => router_trace!("Forwarded VL {}", v),
                Ok(None) => router_debug!("Scheduled no VL"),
                Err(Error::Port(e)) => router_debug!("Port send/receive failed temporarily: {}", e),
                Err(e) => router_debug!("Failed to forward message on VL: {}", e),
            }
        }
    }
}

use log::{error, trace};

use crate::scheduler::TimeSource;
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
    let mut cfg: Config<IN, OUT> = Config::default();
    loop {
        let new_cfg = Configurator::fetch_config(router_config).unwrap();
        if new_cfg != cfg {
            cfg = new_cfg;
        }
        let router = Configurator::reconfigure(&resources, scheduler, &cfg).unwrap();
        match router.forward::<BUF_LEN>(scheduler, time_source) {
            Ok(Some(v)) => trace!("Scheduled VL {v}"),
            Ok(None) => continue,
            Err(e) => error!("{e:?}"),
        }
    }
}

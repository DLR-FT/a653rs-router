use a653rs::prelude::{Name, Partition, PartitionExt, StartContext};
use a653rs_router::prelude::{RouterConfig, RouterState, VirtualLinksConfig};
use a653rs_router_tests::{test_data::CFG, DummyHypervisor, DummyNetIntf};
use core::str::FromStr;
use std::process::exit;

const MTU: usize = 1_000;
const INPUTS: usize = 8;
const OUTPUTS: usize = 8;
const INTERFACES: usize = 8;
const PORTS: usize = 8;
const NAME: &str = "Router";

static mut ROUTER: Option<RouterState<DummyHypervisor, DummyNetIntf, INTERFACES, PORTS>> = None;
static mut VL_CFG: Option<VirtualLinksConfig<INPUTS, OUTPUTS>> = None;

#[derive(Debug)]
struct RouterPartition;

impl Partition<DummyHypervisor> for RouterPartition {
    fn cold_start(&self, ctx: &mut StartContext<DummyHypervisor>) {
        let cfg: RouterConfig<INPUTS, OUTPUTS, INTERFACES, PORTS> =
            serde_yaml::from_str(CFG).unwrap();
        _ = unsafe { VL_CFG.insert(cfg.virtual_links) };
        let router = RouterState::create::<DummyNetIntf>(
            ctx,
            Name::from_str(NAME).unwrap(),
            cfg.interfaces,
            cfg.ports,
            cfg.stack_size,
            entry_point,
        )
        .unwrap();
        _ = unsafe { ROUTER.insert(router) };
        let router = unsafe { ROUTER.as_ref() }.unwrap();
        let cfg = unsafe { VL_CFG.as_ref() }.unwrap().clone();
        println!("router = {router:?}, cfg = {cfg:?}");
        router.start().unwrap();

        // Would run empty loop
        println!("success");
        exit(0)
    }

    fn warm_start(&self, ctx: &mut StartContext<DummyHypervisor>) {
        self.cold_start(ctx)
    }
}

// Not called by DummyHypervisor
extern "C" fn entry_point() {
    let router = unsafe { ROUTER.as_ref() }.unwrap();
    let cfg = unsafe { VL_CFG.as_ref() }.unwrap().clone();
    let router = router.router::<INPUTS, OUTPUTS, MTU>(cfg).unwrap();
    println!("{router:?}")
}

#[test]
fn main() {
    RouterPartition.run();
}

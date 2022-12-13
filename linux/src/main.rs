#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod pseudo;

extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use core::str::FromStr;
use core::time::Duration;
use log::{error, LevelFilter};
use network_partition::prelude::*;
use once_cell::sync::OnceCell;
use pseudo::PseudoInterface;

type Hypervisor = ApexLinuxPartition;

// TODO generate in build.rs with config
// TODO get MTU, PORTS, ... in build.rs
const MTU: PayloadSize = 10_000;
const PORTS: usize = 2;
const LINKS: usize = 2;
const INTERFACES: usize = 1;
const MAX_QUEUE_LEN: usize = 2;
const STACK_SIZE: StackSize = 100_000_000;
const MIN_INTERFACE_DATA_RATE: DataRate = DataRate::b(100_000_000);

static PORT0: OnceCell<SamplingPortDestination<PORT0_MTU, Hypervisor>> = OnceCell::new();
const PORT0_MTU: MessageSize = 10_000;
const PORT0_NAME: &str = "EchoRequest";
const PORT0_REFRESH: Duration = Duration::from_secs(1);

static PORT1: OnceCell<SamplingPortSource<PORT1_MTU, Hypervisor>> = OnceCell::new();
const PORT1_MTU: MessageSize = 10_000;
const PORT1_NAME: &str = "EchoReply";

static IF0: OnceCell<PseudoInterface> = OnceCell::new();
const IF0_MTU: PayloadSize = 10_000;

const VL1_ID: VirtualLinkId = VirtualLinkId::from_u32(1);
const VL1_MTU: PayloadSize = 10_000;

const VL2_ID: VirtualLinkId = VirtualLinkId::from_u32(2);
const VL2_MTU: PayloadSize = 10_000;
const VL2_RATE: DataRate = DataRate::b(10_000);

struct NetworkPartition;

impl Partition<Hypervisor> for NetworkPartition {
    fn cold_start(&self, ctx: &mut StartContext<Hypervisor>) {
        // TODO init ports
        // TODO init interfaces
        // TODO generate based on config
        // PORT_{cfg.name.upper()}.set(ctx.create_sampling_port_destination::<{cfg.msg_size], H>({cfg.name}, {config.validity}).unwrap());
        // INTERFACE_{cfg.name.upper()}.set
        IF0.set(PseudoInterface::new(
            VirtualLinkId::from(2),
            // TODO use maximum of all interfaces MTU for buffer length
            &[1u8; MTU as usize],
            MIN_INTERFACE_DATA_RATE,
        ))
        .unwrap();
        PORT0
            .set(
                ctx.create_sampling_port_destination::<PORT0_MTU>(
                    Name::from_str(PORT0_NAME).unwrap(),
                    PORT0_REFRESH,
                )
                .unwrap(),
            )
            .unwrap();
        PORT1
            .set(
                ctx.create_sampling_port_source::<PORT1_MTU>(Name::from_str(PORT1_NAME).unwrap())
                    .unwrap(),
            )
            .unwrap();

        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::ZERO),
            time_capacity: SystemTime::Infinite,
            entry_point,
            stack_size: STACK_SIZE,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("network_partition").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap()
    }

    fn warm_start(&self, ctx: &mut StartContext<Hypervisor>) {
        self.cold_start(ctx)
    }
}
extern "C" fn entry_point() {
    // TODO generate this all in build.rs
    // Only one shaper is supported at the moment. This should be fine as long as:
    // - all virtual links are emitted on all interfaces
    // - from this follows that the available data rate of the shaper is
    //   the mimum data rate of any attached interface.
    //   Usually, all interfaces have the same data rate.
    let mut shaper = CreditBasedShaper::<LINKS>::new(MIN_INTERFACE_DATA_RATE);

    let mut interfaces: [&dyn Interface; INTERFACES] = [IF0.get().unwrap()];

    let mut vl1 = VirtualLinkData::<VL1_MTU, PORTS, MAX_QUEUE_LEN, Hypervisor>::new(VL1_ID);

    // TODO assert this in build.rs
    // let vl1_config = &config.virtual_links[0];
    // if !vl1_config
    //     .ports
    //     .iter()
    //     .any(|p| p.sampling_port_destination().is_some())
    // {
    vl1.add_port_dst(PORT0.get().unwrap().clone());
    vl1.add_port_src(PORT1.get().unwrap().clone());
    // }

    // TODO assert this in build.rs
    let mut vl2 = VirtualLinkData::<VL2_MTU, PORTS, MAX_QUEUE_LEN, Hypervisor>::new(VL2_ID);
    // let vl2_config = &config.virtual_links[0];
    // if vl2_config
    //     .ports
    //     .iter()
    //     .any(|p| p.sampling_port_destination().is_some())
    // {
    vl2 = vl2.queue(&mut shaper, VL2_RATE);
    // }

    let mut links: [&mut dyn VirtualLink; LINKS] = [&mut vl1, &mut vl2];

    let mut frame_buf: [u8; IF0_MTU as usize] = [0u8; IF0_MTU as usize];
    let mut forwarder = Forwarder::new(&mut frame_buf, &mut shaper, &mut links, &mut interfaces);

    loop {
        if let Err(err) = forwarder.forward::<Hypervisor>() {
            error!("{err:?}");
        }
        Hypervisor::periodic_wait().unwrap();
    }
}

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();

    NetworkPartition.run();
}

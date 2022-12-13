#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod pseudo;

extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use log::{error, trace, LevelFilter};
use network_partition::prelude::*;
use once_cell::sync::OnceCell;
use pseudo::PseudoInterface;
use std::time::Duration;

type Hypervisor = ApexLinuxPartition;

static CONFIG: OnceCell<Config<2, 2, 2>> = OnceCell::new();

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();
    let config =
        serde_yaml::from_str(include_str!("../../config/network_partition_config.yml")).unwrap();
    CONFIG.set(config).unwrap();
    let partition = NetworkPartition::new(
        CONFIG.get().unwrap().stack_size.periodic_process.as_u64() as u32,
        entry_point,
    );
    PartitionExt::<Hypervisor>::run(partition);
}

extern "C" fn entry_point() {
    let config = CONFIG.get().unwrap();

    // TODO generate in build.rs with config
    let mut veth0 = PseudoInterface::new(
        VirtualLinkId::from(1),
        // TODO create buffer with const size from config.[interfaces].mtu
        &[1u8; 1500],
        config.interfaces[0].rate,
    );
    let mut interfaces: [&mut dyn Interface; 1] = [&mut veth0];

    // Only one shaper is supported at the moment. This should be fine as long as:
    // - all virtual links are emitted on all interfaces
    // - from this follows that the available data rate of the shaper is
    //   the mimum data rate of any attached interface.
    //   Usually, all interfaces have the same data rate.
    let mut shaper = CreditBasedShaper::<1>::new(config.interfaces[0].rate);

    // TODO get MTU, PORTS, ... in build.rs
    // TODO how to init sampling ports?
    const MTU: PayloadSize = 10_000;
    const PORTS: usize = 2;
    const MAX_QUEUE_LEN: usize = 2;

    // TODO move onto memory pool?

    let vl1_config = &config.virtual_links[0];
    let mut vl1 = VirtualLinkData::<MTU, PORTS, MAX_QUEUE_LEN, Hypervisor>::new(vl1_config.id);
    if vl1_config
        .ports
        .iter()
        .any(|p| p.sampling_port_destination().is_some())
    {
        vl1 = vl1.queue(&mut shaper, vl1_config.rate);
    }
    // TODO add sampling ports into VL during cold_start

    let mut links: [&mut dyn VirtualLink; 1] = [&mut vl1];

    let mut time = Duration::ZERO;
    loop {
        let time_diff = Hypervisor::get_time().unwrap_duration() - time;
        shaper.restore_all(time_diff).unwrap();
        time = Hypervisor::get_time().unwrap_duration();

        for vl in links.iter_mut() {
            if let Err(err) = vl.receive_hypervisor(&mut shaper) {
                error!("Failed to receive a frame: {err:?}");
            }
        }

        let mut frame_buf = [0u8; 1500];
        for interface in interfaces.iter_mut() {
            if let Ok((vl_id, buf)) = interface.receive(&mut frame_buf) {
                for vl in links.iter_mut() {
                    if vl_id == vl.vl_id() {
                        if let Err(err) = vl.receive_network(buf) {
                            error!("Failed to receive a frame: {err:?}");
                        }
                    }
                }
            }
        }

        let mut transmitted = false;

        while let Some(q_id) = shaper.next_queue() {
            transmitted = true;
            trace!("Attempting transmission from queue {q_id:?}");
            for vl in links.iter_mut() {
                if vl.queue_id() == Some(q_id) {
                    for intf in interfaces.iter_mut() {
                        if let Err(err) = vl.send_network(*intf, &mut shaper) {
                            error!("Failed to send frame to network: {err:?}");
                        }
                    }
                }
            }
        }

        if !transmitted {
            let time_diff = Hypervisor::get_time().unwrap_duration() - time;
            shaper.restore_all(time_diff).unwrap();
        }

        time = Hypervisor::get_time().unwrap_duration();
        Hypervisor::periodic_wait().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use network_partition::prelude::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn parse_code_section_config() {
        // TODO should be configured from config using proc-macro
        let config = include_str!("../../config/network_partition_config.yml");
        let parsed = serde_yaml::from_str::<Config<4, 2, 2>>(config);
        println!("{parsed:?}");
        assert!(parsed.is_ok());
    }
}

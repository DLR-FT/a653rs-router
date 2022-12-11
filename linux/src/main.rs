#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod pseudo;

extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use bytesize::ByteSize;
use log::{error, trace, warn, LevelFilter};
use network_partition::prelude::*;
use once_cell::sync::OnceCell;
use pseudo::PseudoInterface;
use std::time::Duration;

// TODO should be configured from config using proc-macro
const PORT_MTU: MessageSize = 10000;
const PORTS: usize = 2;
const INTERFACES: usize = 1;
const LINKS: usize = 1;
const MAX_QUEUE_LEN: usize = 1;

type Hypervisor = ApexLinuxPartition;

// static LINKS: OnceCell<SamplingVirtualLink<PORT_MTU, 1, 2, ApexLinuxPartition>> = OnceCell::new();
// TODO static PORTS, ...
// TODO VirtualLinks zusammenbasteln anhand von config.
static INTERFACE: OnceCell<PseudoInterface> = OnceCell::new();

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();
    let config = include_str!("../../config/network_partition_config.yml");
    let config = serde_yaml::from_str::<Config<PORTS, LINKS, INTERFACES>>(config)
        .ok()
        .unwrap();
    let partition = NetworkPartition::<PORT_MTU, PORTS, INTERFACES, MAX_QUEUE_LEN, LINKS>::new(
        config,
        entry_point,
    );
    PartitionExt::<Hypervisor>::run(partition);
}

extern "C" fn entry_point() {
    let mut time = Duration::ZERO;
    let mut shaper = CreditBasedShaper::<1>::new(ByteSize::mb(10));
    let if_buffer = [1u8; PORT_MTU as usize];
    let mut interface = PseudoInterface::new(VirtualLinkId::from(1), &if_buffer, ByteSize::mb(100));
    let mut links: heapless::Vec<VirtualLink, 2> = heapless::Vec::default();
    // TODO add VLs here

    loop {
        let time_diff = Hypervisor::get_time().unwrap_duration() - time;
        shaper.restore_all(time_diff).unwrap();
        time = Hypervisor::get_time().unwrap_duration();

        let mut frame_buf = [0u8; PORT_MTU as usize];
        let next_frame = interface.receive(&mut frame_buf).ok();

        for vl in links.iter_mut() {
            let res = match vl {
                VirtualLink::LocalToLocal(vl) => vl.forward_hypervisor(&mut shaper),
                VirtualLink::LocalToNet(vl) => vl.forward_hypervisor(&mut shaper),
                VirtualLink::LocalToNetAndLocal(vl) => vl.forward_hypervisor(&mut shaper),
                VirtualLink::NetToLocal(vl) => match next_frame {
                    Some((vl_id, buf)) => {
                        if vl_id == vl.vl_id() {
                            vl.receive_network(buf)
                        } else {
                            Ok(())
                        }
                    }
                    None => Ok(()),
                },
            };
            if let Err(err) = res {
                error!("Failed to receive a frame: {err:?}");
            }
        }

        // TODO vl.send_network()
        let mut transmitted = false;

        while let Some(q_id) = shaper.next_queue() {
            transmitted = true;
            trace!("Attempting transmission from queue {q_id:?}");
            for vl in links.iter_mut() {
                let res = match vl {
                    VirtualLink::LocalToNet(vl) => {
                        if vl.queue_id() == q_id {
                            // TODO log errors
                            vl.send_network(&mut interface, &mut shaper)
                        } else {
                            Ok(())
                        }
                    }
                    VirtualLink::LocalToNetAndLocal(vl) => {
                        if vl.queue_id() == q_id {
                            vl.send_network(&mut interface, &mut shaper)
                        } else {
                            Ok(())
                        }
                    }
                    _ => Ok(()),
                };
                if let Err(err) = res {
                    error!("Failed to send frame to network: {err:?}");
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

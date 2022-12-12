#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod pseudo;

extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use bytesize::ByteSize;
use log::{error, trace, LevelFilter};
use network_partition::prelude::*;
use pseudo::PseudoInterface;
use std::time::Duration;

type Hypervisor = ApexLinuxPartition;

// TODO generate with decl macro
fn config() -> Config<10, 10, 10> {
    Config::<10, 10, 10> {
        stack_size: StackSizeConfig {
            periodic_process: ByteSize::kb(100),
        },
        ports: heapless::Vec::from_slice(&[
            Port::SamplingPortDestination(SamplingPortDestinationConfig {
                channel: heapless::String::from("EchoRequest"),
                msg_size: ByteSize::kb(2),
                validity: Duration::from_secs(1),
                virtual_link: 0,
            }),
            Port::SamplingPortSource(SamplingPortSourceConfig {
                channel: heapless::String::from("EchoReply"),
                msg_size: ByteSize::kb(2),
                virtual_link: 1,
            }),
        ])
        .unwrap(),
        virtual_links: heapless::Vec::from_slice(&[
            VirtualLinkConfig {
                id: 0,
                rate: DataRate::b(1000),
                msg_size: ByteSize::kb(1),
                interfaces: heapless::Vec::from_slice(&[
                    InterfaceName::from("veth0"),
                    InterfaceName::from("veth1"),
                ])
                .unwrap(),
            },
            VirtualLinkConfig {
                id: 1,
                rate: DataRate::b(1000),
                msg_size: ByteSize::kb(1),
                interfaces: heapless::Vec::from_slice(&[]).unwrap(),
            },
        ])
        .unwrap(),
        interfaces: heapless::Vec::from_slice(&[InterfaceConfig {
            name: InterfaceName::from("veth0"),
            rate: DataRate::b(10000000),
            mtu: ByteSize::kb(1),
        }])
        .unwrap(),
    }
}

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();
    let config = config();
    let partition = NetworkPartition::new(
        config.stack_size.periodic_process.as_u64() as u32,
        entry_point,
    );
    PartitionExt::<Hypervisor>::run(partition);
}

extern "C" fn entry_point() {
    let config = config();
    let mut time = Duration::ZERO;
    let if_buffer = [1u8; 1500];
    //config().interfaces.get(0).mtu as usize];

    // TODO generate interfaces from config
    let mut interface = PseudoInterface::new(
        VirtualLinkId::from(1),
        &if_buffer,
        config.interfaces[0].rate,
    );
    let mut interfaces: heapless::Vec<&mut dyn Interface, 1> = heapless::Vec::default();
    if interfaces.push(&mut interface).is_err() {
        panic!("Failed to add interface");
    }

    // TODO convert back to builder pattern for init'ing queues
    let mut shaper = CreditBasedShaper::<1>::new(config.interfaces[0].rate);
    // TODO create VLs from config with generated interfaces, ports, and queues
    let mut links: heapless::Vec<&mut dyn VirtualLink, 2> = heapless::Vec::default();
    //links.push(&VirtualLinkData)
    // for vl in config.virtual_links.iter() {
    //     let is_net = !vl.interfaces.is_empty();
    //     let is_local = config.ports.iter().any(|p| {
    //         if let Some(source) = p.sampling_port_source() {
    //             source.virtual_link == vl.id
    //         } else {
    //             false
    //         }
    //     });
    //     let link = if is_net {
    //         VirtualLink::LocalToNet()
    //     }
    // }
    // let vl0 = VirtualLinkData::new(VirtualLinkId::from(0))
    // .queue(&mut shaper, DataRate::b(10), QueueId::from(0))
    // links.push(LocalToLocalAndNetVirtualLink::)

    // TODO generate shaper share config and number of queues

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

#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use bytesize::ByteSize;
use heapless::{spsc::Queue, LinearMap};
use log::{error, trace, LevelFilter};
use network_partition::prelude::*;
use once_cell::sync::OnceCell;

type Hypervisor = ApexLinuxPartition;

// TODO should be configured from config using proc-macro
const PORT_MTU: MessageSize = 10000;
const TABLE_SIZE: usize = 10;
const QUEUE_CAPACITY: usize = 2;
const INTERFACES: usize = 1;

// TODO use once big OnceCell<struct>
static CONFIG: OnceCell<Config<TABLE_SIZE, TABLE_SIZE, INTERFACES>> = OnceCell::new();
static ROUTER: OnceCell<Router<TABLE_SIZE>> = OnceCell::new();
static SOURCE_PORTS: OnceCell<
    LinearMap<ChannelId, SamplingPortSource<PORT_MTU, Hypervisor>, TABLE_SIZE>,
> = OnceCell::new();
static DESTINATION_PORTS: OnceCell<
    LinearMap<ChannelId, SamplingPortDestination<PORT_MTU, Hypervisor>, TABLE_SIZE>,
> = OnceCell::new();

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();
    let config = include_str!("../../config/network_partition_config.yml");
    let parsed_config = serde_yaml::from_str::<Config<TABLE_SIZE, TABLE_SIZE, INTERFACES>>(config);
    if let Err(error) = parsed_config {
        error!("{error:?}");
        panic!();
    }
    CONFIG.set(parsed_config.ok().unwrap()).unwrap();
    trace!("Have config: {CONFIG:?}");
    let partition = NetworkPartition::<PORT_MTU, TABLE_SIZE, INTERFACES, Hypervisor>::new(
        CONFIG.get().unwrap().clone(),
        &ROUTER,
        &SOURCE_PORTS,
        &DESTINATION_PORTS,
        entry_point,
    );
    partition.run();
}

extern "C" fn entry_point() {
    // TODO move to partition module
    let interface = PseudoInterface::<PORT_MTU>::new(
        Frame::new(VirtualLinkId::from(1), [1u8; PORT_MTU as usize]),
        ByteSize::kb(10),
    );
    let echo_allowed_rate = ByteSize::kb(1);
    //let interface = UdpSender::new(socket, echo_allowed_rate);
    let router = ROUTER.get().unwrap();
    let port_dsts = DESTINATION_PORTS.get().unwrap();
    let port_srcs = SOURCE_PORTS.get().unwrap();
    let mut shaper = CreditBasedShaper::<1>::new(ByteSize::mb(10));
    let echo_queue = shaper.add_queue(echo_allowed_rate).unwrap();
    let mut queues: LinearMap<QueueId, Queue<Frame<PORT_MTU>, QUEUE_CAPACITY>, TABLE_SIZE> =
        LinearMap::default();
    queues.insert(echo_queue, Queue::default()).unwrap();

    // TODO init ports

    loop {
        for dst in port_dsts {
            let mut message = Message::<PORT_MTU>::default();
            let res = dst
                .receive_message(&mut message)
                .and_then(|m| m.get_virtual_link(router))
                .and_then(|link| {
                    link.forward_message(&message, port_srcs, &mut queues, &mut shaper)
                });
            if res.is_err() {
                error!("Failed to deliver frame to all destinations: {res:?}");
            }
        }
        // for i in interfaces
        // TODO there is no port configured as a destination for VL 1
        let mut frame = Frame::<PORT_MTU>::default();
        let res = interface
            .receive_frame(&mut frame)
            .and_then(|f| f.get_virtual_link(router))
            .and_then(|f| f.forward_frame(&frame, port_srcs));

        if let Err(err) = res {
            error!("Failed to forward frame: {err:?}");
        }

        let mut frames_transmitted = 0;
        while let Some(q_id) = shaper.next_queue() {
            // transmit
            let q = queues.get_mut(&q_id).unwrap();
            let frame = FrameQueue::dequeue_frame(q).unwrap();
            let transmission = interface.send_frame(q_id, &frame);
            let pl_size = frame.len();
            let transmission = match transmission {
                Ok(transmission) => transmission,
                Err(transmission) => {
                    error!("Transmission failed: {frame:?}");
                    transmission
                }
            };
            // Act like we transmitted the complete frame. It does not need to be registered to the shaper
            // because it was already dequeued and retransmissions are not yet supported.
            let transmission = transmission.with_size(ByteSize::b(frame.len() as u64));
            trace!("Transmition: {transmission:?}, Payload size: {pl_size:?}");
            shaper.record_transmission(transmission).unwrap();
            frames_transmitted += 1;
            trace!("Frames transmitted: {frames_transmitted:?}");
        }

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
        let parsed = serde_yaml::from_str::<Config<4, 2, 1>>(config);
        println!("{parsed:?}");
        assert!(parsed.is_ok());
    }
}

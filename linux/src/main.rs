extern crate log;

use std::time::Duration;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use bytesize::ByteSize;
use heapless::{spsc::Queue, LinearMap};
use log::{error, trace, LevelFilter};
use network_partition::prelude::{Error, *};
use once_cell::sync::OnceCell;

type Hypervisor = ApexLinuxPartition;

// TODO should be configured from config using proc-macro
const PORT_MTU: MessageSize = 10000;
const TABLE_SIZE: usize = 10;
const QUEUE_CAPACITY: usize = 2;

// TODO use once big OnceCell<struct>
static CONFIG: OnceCell<Config> = OnceCell::new();
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
    let parsed_config = serde_yaml::from_str::<Config>(config);
    if let Err(error) = parsed_config {
        error!("{error:?}");
        panic!();
    }
    CONFIG.set(parsed_config.ok().unwrap()).unwrap();
    trace!("Have config: {CONFIG:?}");
    let partition = NetworkPartition::<PORT_MTU, TABLE_SIZE, Hypervisor>::new(
        CONFIG.get().unwrap().clone(),
        &ROUTER,
        &SOURCE_PORTS,
        &DESTINATION_PORTS,
        entry_point,
    );
    partition.run();
}

fn process_destination_port<'a, H: ApexSamplingPortP4>(
    port: &'a SamplingPortDestination<PORT_MTU, H>,
    router: &'a dyn RouteLookup<TABLE_SIZE>,
    srcs: &'a dyn SamplingPortLookup<PORT_MTU, H>,
    queues: &'a mut dyn QueueLookup<PORT_MTU>,
    shaper: &'a mut dyn Shaper,
) -> Result<(), Error> {
    let mut frame = Frame::<PORT_MTU>::default();
    let frame = port.receive_frame(&mut frame)?;
    let link = frame.get_virtual_link(router)?;
    link.forward_sampling_port(frame, srcs, queues, shaper)
}

extern "C" fn entry_point() {
    // TODO move to partition module
    let router = ROUTER.get().unwrap();
    let port_dsts = DESTINATION_PORTS.get().unwrap();
    let port_srcs = SOURCE_PORTS.get().unwrap();
    let mut shaper = CreditBasedShaper::<1>::new(ByteSize::mb(10));
    let echo_queue = shaper.add_queue(ByteSize::kb(1)).unwrap();
    let mut queues: LinearMap<QueueId, Queue<Frame<PORT_MTU>, QUEUE_CAPACITY>, TABLE_SIZE> =
        LinearMap::default();
    queues.insert(echo_queue, Queue::default()).unwrap();

    loop {
        for (_, dst) in port_dsts {
            let res = process_destination_port(dst, router, port_srcs, &mut queues, &mut shaper);
            if res.is_err() {
                error!("Failed to deliver frame to all destinations: {res:?}");
            }
        }
        let mut frames_transmitted = 0;
        while let Some(q_id) = shaper.next_queue() {
            // transmit
            let q = queues.get_mut(&q_id).unwrap();
            let frame = FrameQueue::dequeue_frame(q).unwrap();
            // TODO q.send_network
            let transmission = Transmission::for_frame(q_id, &frame);
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
        let parsed = serde_yaml::from_str::<Config>(config);
        assert!(parsed.is_ok());
    }
}

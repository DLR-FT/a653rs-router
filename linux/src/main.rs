extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use heapless::LinearMap;
use log::{error, trace, warn, LevelFilter};
use network_partition::prelude::{Error, *};
use once_cell::sync::OnceCell;

type Hypervisor = ApexLinuxPartition;

// TODO should be configured from config using proc-macro
const PORT_MTU: MessageSize = 10000;
const TABLE_SIZE: usize = 10;

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

extern "C" fn entry_point() {
    // TODO move to partition module
    let router = ROUTER.get().unwrap();
    let dst_ports = DESTINATION_PORTS.get().unwrap();
    let src_ports = SOURCE_PORTS.get().unwrap();

    // TODO map from virtual link to network interface for multiple interfaces

    loop {
        //     // Read from all ports and enqueue
        //     // let received = port.receive();
        //     // try queue.enqueue(received);
        //     let next_frame = shaper.try_get_next_frame::<FRAME_SIZE>(&mut queue);
        //     match next_frame {
        //         Ok(_) => {
        //             let result = router.route_local_output::<ECHO_PORT_SIZE_BYTES>(&input);
        //             match result {
        //                 Ok(_) => {
        //                     trace!("Replied to echo")
        //                 }
        //                 Err(err) => {
        //                     error!("Failed to reply to echo: {err:?}")
        //                 }
        //             }
        //         }
        //         Err(err) => {
        //             error!("{err:?}");
        //         }
        //     }
        //     shaper.restore(&mut queue);

        // TODO refactor

        let do_collect_dst_port = |id: &ChannelId,
                                   port: &SamplingPortDestination<PORT_MTU, Hypervisor>|
         -> Result<(), Error> {
            let mut buffer = [0u8; PORT_MTU as usize];

            let (valid, _) = port.receive(&mut buffer)?;
            if valid == Validity::Invalid {
                return Err(Error::InvalidData);
            }
            let dst_address = router.route_local_output(id)?;

            // Try to find local ports that want are part of virtual link and deliver immediately.
            if let Ok(local_ports) = router.route_remote_input(&dst_address) {
                for port_id in local_ports {
                    let port = src_ports.get(&port_id);
                    if port.is_none() {
                        warn!("Port with id {port_id:?} not initialized");
                    } else {
                        let send_result = port.unwrap().send(&buffer);
                        if send_result.is_err() {
                            error!(
                                "Failed to send to port {port_id:?}: {:?}",
                                send_result.err().unwrap()
                            );
                        }
                    }
                }
            }

            //let frame = Frame::new(dst_address, buffer);
            // TODO do shaping before that
            //let res = network_interface.send_frame(&frame);
            //if res.is_err() {
            //// TODO store error and return all stored errors as result
            //error!(
            //"Failed to send frame to link for virtual link {dst_address:?}: {:?}",
            //res.err().unwrap()
            //);
            //}

            //let _frame = Frame::<FRAME_PAYLOAD_SIZE>::from(&buffer);
            // TODO submit including virtual link tag and sequence number
            // virtual_links[&dst_address].queue.enqueue(frame)?;

            Ok(())
        };

        for (id, port) in dst_ports.iter() {
            if let Err(err) = do_collect_dst_port(id, port) {
                error!("{err:?}");
            }
        }

        let do_collect_network_port = || -> Result<(), Error> {
            //let mut buffer = [0u8; FRAME_MTU];
            //let res = network_interface.receive_frame(&mut buffer);
            //if res.is_err() {
            //    // TODO store error and return it
            //} else {
            //    let frame = res.unwrap();
            //    let link = frame.link();
            //    let payload = frame.payload();
            //    let local_ports = router.route_remote_input(&link);
            //    if local_ports.is_err() {
            //        // TODO store error and return it
            //    } else {
            //        for port in local_ports.ok().unwrap() {
            //            // TODO handle error
            //            src_ports.get(&port).unwrap().send(&payload).unwrap();
            //        }
            //    }
            //}
            Ok(())
        };

        if let Err(err) = do_collect_network_port() {
            error!("{err:?}");
        }

        // let do_submit_network_port =
        //     |_port: &FakeNetworkInterface<FRAME_MTU>| -> Result<(), Error> {
        //         // TODO apply shaping to queues of network ports and send frames to network
        //         Ok(())
        //     };

        // for queue in queues {
        // if let Err(err) = do_submit_queue_port(port) {
        // error!("{err:?}");
        // }
        // }

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

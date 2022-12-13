use core::time::Duration;

use crate::{network::Interface, shaper::Shaper, virtual_link::VirtualLink};
use apex_rs::prelude::*;
use log::error;

/// Forwards frames between the hypervisor and the network and between ports on the same hypervisor.
#[derive(Debug)]
pub struct Forwarder<'a> {
    timestamp: Duration,
    frame_buf: &'a mut [u8],
    shaper: &'a mut dyn Shaper,
    links: &'a mut [&'a mut dyn VirtualLink],
    interfaces: &'a mut [&'a dyn Interface],
}

impl<'a> Forwarder<'a> {
    /// Creates a new `Forwarder`.
    pub fn new(
        frame_buf: &'a mut [u8],
        shaper: &'a mut dyn Shaper,
        links: &'a mut [&'a mut dyn VirtualLink],
        interfaces: &'a mut [&'a dyn Interface],
    ) -> Self {
        Self {
            timestamp: Duration::ZERO,
            frame_buf,
            shaper,
            links,
            interfaces,
        }
    }
    /// TODO create struct for parameters
    pub fn forward<H: ApexTimeP4Ext>(&mut self) -> Result<(), Error> {
        let time = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
        let time_diff = time - self.timestamp;
        self.timestamp = time;

        self.shaper.restore_all(time_diff).unwrap();

        for vl in self.links.iter_mut() {
            if let Err(err) = vl.receive_hypervisor(self.shaper) {
                error!("Failed to receive a frame: {err:?}");
            }
        }

        for interface in self.interfaces.iter_mut() {
            if let Ok((vl_id, buf)) = interface.receive(self.frame_buf) {
                for vl in self.links.iter_mut() {
                    if vl_id == vl.vl_id() {
                        if let Err(err) = vl.receive_network(buf) {
                            error!("Failed to receive a frame: {err:?}");
                        }
                    }
                }
            }
        }

        let mut transmitted = false;

        while let Some(q_id) = self.shaper.next_queue() {
            transmitted = true;
            // TODO report trace!("Attempting transmission from queue {q_id:?}");
            for vl in self.links.iter_mut() {
                if vl.queue_id() == Some(q_id) {
                    for intf in self.interfaces.iter_mut() {
                        if let Err(err) = vl.send_network(*intf, self.shaper) {
                            error!("Failed to send frame to network: {err:?}");
                        }
                    }
                }
            }
        }

        if !transmitted {
            let time_diff = <H as ApexTimeP4Ext>::get_time().unwrap_duration() - time;
            self.shaper.restore_all(time_diff).unwrap();
        }

        Ok(())
    }
}

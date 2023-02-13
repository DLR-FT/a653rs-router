use core::{fmt::Debug, time::Duration};

use crate::{
    error::Error,
    network::NetworkInterface,
    prelude::{PayloadSize, PlatformNetworkInterface, VirtualLinkId},
    shaper::Shaper,
    virtual_link::VirtualLink,
};
use apex_rs::prelude::*;
use log::{error, trace};

/// Trait that hides hypervisor and MTU.
pub trait Interface: Debug {
    /// Send data.
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<Duration, Duration>;

    /// Receive data.
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<(VirtualLinkId, &'a [u8]), Error>;
}

impl<const MTU: PayloadSize, H: PlatformNetworkInterface + Debug> Interface
    for NetworkInterface<MTU, H>
{
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<(VirtualLinkId, &'a [u8]), Error> {
        NetworkInterface::receive(self, buf)
    }

    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<Duration, Duration> {
        NetworkInterface::send(self, vl, buf)
    }
}

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

    /// Forwards messages between the hypervisor and the network.
    pub fn forward<H: ApexTimeP4Ext>(&mut self) -> Result<(), Error> {
        let time = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
        let time_diff = time - self.timestamp;
        self.timestamp = time;
        let mut last_err: Option<Error> = None;
        trace!("Restoring queue credit");
        if let Err(err) = self.shaper.restore_all(time_diff) {
            last_err = Some(err);
        }
        trace!("Receiving messages from hypervisor");
        if let Err(err) = self.receive_hypervisor() {
            last_err = Some(err);
        }
        trace!("Receiving messages from the network");
        if let Err(err) = self.receive_network() {
            last_err = Some(err);
        }
        trace!("Sending messages to the network");
        let (_, transmitted) = match self.send_network() {
            Ok(transmitted) => (None, transmitted),
            Err((transmitted, err)) => (Some(err), transmitted),
        };
        if !transmitted {
            trace!(
                "Restoring credit to queues, because there were no transmissions to the network."
            );
            let time_diff = <H as ApexTimeP4Ext>::get_time().unwrap_duration() - time;
            if let Err(err) = self.shaper.restore_all(time_diff) {
                last_err = Some(err);
            }
        }
        match last_err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    fn receive_hypervisor(&mut self) -> Result<(), Error> {
        let mut err: Option<Error> = None;
        for vl in self.links.iter_mut() {
            if let Err(e) = vl.receive_hypervisor(self.shaper) {
                error!("VL {} {e}", vl.vl_id());
                err = Some(e);
            }
        }

        match err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    fn receive_network(&mut self) -> Result<(), Error> {
        self.interfaces
            .iter_mut()
            .filter_map(|intf| {
                let res = intf.receive(self.frame_buf);
                if let Ok((vl_id, buf)) = res {
                    self.links
                        .iter_mut()
                        .find(|vl| vl.vl_id() == vl_id)
                        .and_then(|vl| vl.receive_network(buf).err())
                        .map(|e| {
                            error!("{e}");
                            Err(e)
                        })
                } else {
                    Some(Err(res.unwrap_err()))
                }
            })
            .last()
            .unwrap_or(Ok(()))
    }

    fn send_network(&mut self) -> Result<bool, (bool, Error)> {
        let mut transmitted = false;

        let mut last: Option<Error> = None;
        while let Some(q_id) = self.shaper.next_queue() {
            trace!("Next queue is {}", q_id);
            transmitted = true;
            let error: Option<Error> = self
                .links
                .iter_mut()
                .find(|vl| vl.queue_id() == Some(q_id))
                .and_then(|vl| {
                    self.interfaces
                        .iter_mut()
                        .filter_map(|intf| vl.send_network(*intf, self.shaper).err())
                        .map(|e| {
                            error!("{e}");
                            e
                        })
                        .last()
                });
            if let Some(e) = error {
                last = Some(e);
            }
        }

        if let Some(err) = last {
            Err((transmitted, err))
        } else {
            Ok(transmitted)
        }
    }
}

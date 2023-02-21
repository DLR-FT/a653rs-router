use core::fmt::Debug;

use crate::{
    error::Error,
    network::NetworkInterface,
    prelude::{InterfaceError, PayloadSize, PlatformNetworkInterface, Scheduler, VirtualLinkId},
    virtual_link::VirtualLink,
};
use apex_rs::prelude::{ApexTimeP4Ext, SystemTime};
use log::{error, trace, warn};

/// Trait that hides hypervisor and MTU.
pub trait Interface: Debug {
    /// Send data.
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<usize, Error>;

    /// Receive data.
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<(VirtualLinkId, &'a [u8]), Error>;
}

impl<const MTU: PayloadSize, H: PlatformNetworkInterface + Debug> Interface
    for NetworkInterface<MTU, H>
{
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<(VirtualLinkId, &'a [u8]), Error> {
        NetworkInterface::receive(self, buf)
    }

    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<usize, Error> {
        NetworkInterface::send(self, vl, buf)
    }
}

/// Forwards frames between the hypervisor and the network and between ports on the same hypervisor.
#[derive(Debug)]
pub struct Forwarder<'a> {
    scheduler: &'a mut dyn Scheduler,
    links: &'a mut [&'a dyn VirtualLink],
    interfaces: &'a mut [&'a dyn Interface],
}

impl<'a> Forwarder<'a> {
    /// Creates a new `Forwarder`.
    pub fn new(
        scheduler: &'a mut dyn Scheduler,
        links: &'a mut [&'a dyn VirtualLink],
        interfaces: &'a mut [&'a dyn Interface],
    ) -> Self {
        Self {
            scheduler,
            links,
            interfaces,
        }
    }

    /// Forwards messages between the hypervisor and the network.
    pub fn forward<H: ApexTimeP4Ext>(&mut self, buf: &mut [u8]) {
        for intf in self.interfaces.iter() {
            match intf.receive(buf) {
                Ok((vl, data)) => {
                    trace!("Received: {data:?}");
                    for vl in self.links.iter().filter(|l| l.vl_id() == vl) {
                        if let Err(e) = vl.process_remote(data) {
                            warn!("Failed to process message: {e}")
                        }
                    }
                }
                Err(Error::InterfaceReceiveFail(InterfaceError::NoData)) => {}
                Err(e) => {
                    warn!("Failed to read from interface: {e}");
                }
            }
        }
        if let SystemTime::Normal(time) = <H as ApexTimeP4Ext>::get_time() {
            if let Some(next) = self.scheduler.next(time) {
                trace!("Scheduled VL {next}");
                if let Some(next) = self.links.iter().find(|l| l.vl_id() == next) {
                    if let Ok(data) = next.read_local(buf) {
                        // TODO only forward to the interfaces for the VL
                        for i in self.interfaces.iter() {
                            trace!("Sending to network: {data:?}");
                            if let Err(e) = i.send(&next.vl_id(), data) {
                                warn!("Failed to send to interface {e}")
                            }
                        }
                    }
                }
            } else {
                //trace!("Scheduled no VL");
            }
        } else {
            error!("System time was not normal")
        }
    }
}

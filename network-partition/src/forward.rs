use core::fmt::Debug;

use crate::{
    error::Error,
    prelude::{
        InterfaceError, IoScheduler, NetworkInterface, NetworkInterfaceId, PayloadSize,
        PlatformNetworkInterface, VirtualLinkId,
    },
    virtual_link::VirtualLink,
};
use a653rs::prelude::{ApexTimeP4Ext, SystemTime};
use log::{error, trace, warn};
use small_trace::small_trace;

/// Trait that hides hypervisor and MTU.
pub trait Interface: Debug {
    /// Returns the ID of the network interface.
    fn id(&self) -> NetworkInterfaceId;

    /// Send data.
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<usize, Error>;

    /// Receive data.
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<(VirtualLinkId, &'a [u8]), Error>;
}

impl<const MTU: PayloadSize, H: PlatformNetworkInterface + Debug> Interface
    for NetworkInterface<MTU, H>
{
    fn id(&self) -> NetworkInterfaceId {
        NetworkInterface::id(self)
    }

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
    scheduler: &'a mut dyn IoScheduler,
    links: &'a mut [&'a dyn VirtualLink],
    interfaces: &'a mut [&'a dyn Interface],
}

impl<'a> Forwarder<'a> {
    /// Creates a new `Forwarder`.
    pub fn new(
        scheduler: &'a mut dyn IoScheduler,
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
                    small_trace!(begin_forward_from_network, vl.0 as u16);
                    trace!("Received: {data:?}");
                    for vl in self.links.iter().filter(|l| l.vl_id() == vl) {
                        if let Err(e) = vl.process_remote(data) {
                            warn!("Failed to process message: {e}")
                        }
                    }
                    small_trace!(end_forward_from_network, vl.0 as u16);
                }
                Err(Error::InterfaceReceiveFail(InterfaceError::NoData)) => {}
                Err(e) => {
                    warn!("Failed to read from interface: {e}");
                }
            }
        }
        if let SystemTime::Normal(time) = <H as ApexTimeP4Ext>::get_time() {
            if let Some(next) = self.scheduler.schedule_next(&time) {
                small_trace!(begin_virtual_link_scheduled, next.0 as u16);
                trace!("Scheduled VL {next}");
                if let Some(next) = self.links.iter().find(|l| l.vl_id() == next) {
                    if let Ok(data) = next.read_local(buf) {
                        small_trace!(begin_forward_from_apex, next.vl_id().0 as u16);
                        for i in self.interfaces.iter().filter(|i| next.connects_to(&i.id())) {
                            trace!("Sending to network: {data:?}");
                            small_trace!(begin_forward_to_network, next.vl_id().0 as u16);
                            if let Err(e) = i.send(&next.vl_id(), data) {
                                warn!("Failed to send to interface {e}")
                            }
                            small_trace!(end_forward_to_network, next.vl_id().0 as u16);
                        }
                        small_trace!(end_forward_from_apex, next.vl_id().0 as u16);
                    }
                }
                small_trace!(end_virtual_link_scheduled, next.0 as u16);
            } else {
                //info!("Scheduled no VL");
            }
        } else {
            error!("System time was not normal")
        }
    }
}

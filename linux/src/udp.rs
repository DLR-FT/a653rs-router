use std::time::Duration;
use std::{mem::size_of, net::UdpSocket};

use log::{error, trace, warn};
use network_partition::prelude::{DataRate, Error, Interface, VirtualLinkId};

#[derive(Debug)]
pub struct UdpInterface<const MTU: usize> {
    sock: UdpSocket,
    rate: DataRate,
}

impl<const MTU: usize> UdpInterface<MTU> {
    pub fn new(sock: UdpSocket, rate: DataRate) -> UdpInterface<MTU> {
        Self { sock, rate }
    }
}

impl<const MTU: usize> Interface for UdpInterface<MTU> {
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<(VirtualLinkId, &'a [u8]), Error> {
        match self.sock.recv(buf) {
            Ok(read) => {
                let vl_id_len = size_of::<VirtualLinkId>();
                let vl_id = &buf[0..vl_id_len];
                let mut vl_id_buf = [0u8; size_of::<VirtualLinkId>()];
                vl_id_buf.copy_from_slice(vl_id);
                let vl_id = u32::from_be_bytes(vl_id_buf);
                let vl_id = VirtualLinkId::from_u32(vl_id);
                trace!("Received message from UDP socket for VL {vl_id}");
                Ok((vl_id, &buf[vl_id_len..read]))
            }
            Err(err) => {
                warn!("Failed to receive from UDP socket: {err:?}");
                Err(Error::InterfaceReceiveFail)
            }
        }
    }

    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<Duration, Duration> {
        let id = vl.into_inner();
        let id = id.to_be_bytes();
        let udp_buf = [id.as_slice(), buf].concat();
        match self.sock.send(&udp_buf) {
            Ok(trans) => {
                trace!("Send {} bytes to UDP socket", udp_buf.len());
                Ok(self.duration(trans))
            }
            Err(err) => {
                error!("Failed to send to UDP socket: {err:?}");
                Err(Duration::ZERO)
            }
        }
    }
}

impl<const MTU: usize> UdpInterface<MTU> {
    fn duration(&self, trans: usize) -> Duration {
        let duration = trans as f64 * 8.0 / self.rate.as_f64() * 1_000_000_000.0;
        Duration::from_nanos(duration as u64)
    }
}

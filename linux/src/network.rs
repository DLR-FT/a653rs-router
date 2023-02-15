use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{mem::size_of, net::UdpSocket};

use apex_rs_linux::partition::ApexLinuxPartition;

use log::{error, trace, warn};
use network_partition::prelude::{
    CreateNetworkInterfaceId, DataRate, Error, InterfaceError, NetworkInterfaceId,
    PlatformNetworkInterface, VirtualLinkId,
};
use once_cell::sync::Lazy;

#[derive(Debug)]
pub struct LinuxNetworking;

static SOCKETS: Lazy<Arc<Mutex<Vec<UdpSocket>>>> =
    Lazy::new(|| Arc::new(Mutex::new(ApexLinuxPartition::receive_udp_sockets())));

static INTERFACES: Lazy<Arc<Mutex<Vec<LimitedUdpSocket>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

impl PlatformNetworkInterface for LinuxNetworking {
    fn platform_interface_receive_unchecked(
        id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<(VirtualLinkId, &'_ [u8]), Error> {
        let index: usize = id.into();
        let interfaces = INTERFACES.lock().unwrap();
        let sock = interfaces
            .get(index)
            .ok_or(Error::InterfaceReceiveFail(InterfaceError::NotFound))?;
        match sock.sock.recv(buffer) {
            Ok(read) => {
                let vl_id_len = size_of::<VirtualLinkId>();
                let vl_id = &buffer[0..vl_id_len];
                let mut vl_id_buf = [0u8; size_of::<VirtualLinkId>()];
                vl_id_buf.copy_from_slice(vl_id);
                let vl_id = u32::from_be_bytes(vl_id_buf);
                let vl_id = VirtualLinkId::from_u32(vl_id);
                let msg = &buffer[vl_id_len..read];
                trace!("Received message from UDP socket for VL {vl_id}: {:?}", msg);
                Ok((vl_id, msg))
            }
            Err(err) => {
                warn!("Failed to receive from UDP socket: {err:?}");
                Err(Error::InterfaceReceiveFail(InterfaceError::NoData))
            }
        }
    }

    fn platform_interface_send_unchecked(
        id: network_partition::prelude::NetworkInterfaceId,
        vl: VirtualLinkId,
        buffer: &[u8],
    ) -> Result<Duration, Duration> {
        let index: usize = id.into();
        let interfaces = INTERFACES.lock().unwrap();
        let sock = interfaces.get(index).ok_or(Duration::ZERO)?;
        let id = vl.into_inner();
        let id = id.to_be_bytes();
        let udp_buf = [id.as_slice(), buffer].concat();
        match sock.sock.send(&udp_buf) {
            Ok(trans) => {
                trace!("Send {} bytes to UDP socket", udp_buf.len());
                Ok(sock.duration(trans))
            }
            Err(err) => {
                error!("Failed to send to UDP socket: {err:?}");
                Err(Duration::ZERO)
            }
        }
    }
}

#[derive(Debug)]
struct LimitedUdpSocket {
    sock: UdpSocket,
    rate: DataRate,
}

impl LimitedUdpSocket {
    /// trans is transmitted bytes
    fn duration(&self, trans: usize) -> Duration {
        // TODO configure data rate from configuration
        let duration = ((trans as u64) * 8_000_000_000) / self.rate.as_u64();
        Duration::from_nanos(duration)
    }
}

impl CreateNetworkInterfaceId<LinuxNetworking> for LinuxNetworking {
    fn create_network_interface_id(
        _name: &str, // TODO use network_partition_config::config::InterfaceName ?
        destination: &str,
        rate: DataRate,
    ) -> Result<NetworkInterfaceId, Error> {
        let mut interfaces = INTERFACES.lock().unwrap(); // TODO wrap error
        let sock = SOCKETS.lock().unwrap().pop().unwrap(); // TODO wrap error
        sock.set_nonblocking(true).unwrap();
        sock.connect(destination).unwrap();
        let sock = LimitedUdpSocket { sock, rate };
        interfaces.push(sock);
        let id = interfaces.len() - 1;

        Ok(NetworkInterfaceId::from(id))
    }
}

#[cfg(test)]
mod tests {
    use network_partition::prelude::DataRate;
    use std::{net::UdpSocket, time::Duration};

    use super::LimitedUdpSocket;

    #[test]
    fn calc_duration() {
        let sock = UdpSocket::bind("127.0.0.1:34254").unwrap();
        let limited_sock = LimitedUdpSocket {
            sock,
            rate: DataRate::b(100),
        };
        let trans = 100;
        assert_eq!(limited_sock.duration(trans), Duration::from_secs(8))
    }
}

use std::sync::{Arc, Mutex};
use std::{mem::size_of, net::UdpSocket};

use apex_rs_linux::partition::ApexLinuxPartition;

use log::{error, trace, warn};
use network_partition::prelude::{
    CreateNetworkInterfaceId, DataRate, InterfaceError, NetworkInterfaceId,
    PlatformNetworkInterface, UdpInterfaceConfig, VirtualLinkId,
};
use once_cell::sync::Lazy;
use small_trace::*;

#[derive(Debug)]
pub struct UdpNetworkInterface;

static SOCKETS: Lazy<Arc<Mutex<Vec<UdpSocket>>>> =
    Lazy::new(|| Arc::new(Mutex::new(ApexLinuxPartition::receive_udp_sockets())));

static INTERFACES: Lazy<Arc<Mutex<Vec<LimitedUdpSocket>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

impl PlatformNetworkInterface for UdpNetworkInterface {
    type Configuration = UdpInterfaceConfig;

    fn platform_interface_receive_unchecked(
        id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<(VirtualLinkId, &'_ [u8]), InterfaceError> {
        let index: usize = id.into();
        let interfaces = INTERFACES.lock().unwrap();
        let sock = interfaces.get(index).ok_or(InterfaceError::NotFound)?;
        match sock.sock.recv(buffer) {
            Ok(read) => {
                gpio_trace!(TraceEvent::NetworkReceive(id.0 as u16));
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
                Err(InterfaceError::NoData)
            }
        }
    }

    fn platform_interface_send_unchecked(
        netid: NetworkInterfaceId,
        vl: VirtualLinkId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError> {
        let index: usize = netid.into();
        let interfaces = INTERFACES.lock().unwrap();
        let sock = interfaces.get(index).ok_or(InterfaceError::NotFound)?;
        let id = vl.into_inner();
        let id = id.to_be_bytes();
        let udp_buf = [id.as_slice(), buffer].concat();
        match sock.sock.send(&udp_buf) {
            Ok(trans) => {
                gpio_trace!(TraceEvent::NetworkSend(netid.0 as u16));
                trace!("Send {} bytes to UDP socket", udp_buf.len());
                Ok(trans)
            }
            Err(err) => {
                error!("Failed to send to UDP socket: {err:?}");
                Err(InterfaceError::SendFailed)
            }
        }
    }
}

#[derive(Debug)]
struct LimitedUdpSocket {
    sock: UdpSocket,
    _rate: DataRate,
}

impl CreateNetworkInterfaceId<UdpNetworkInterface> for UdpNetworkInterface {
    fn create_network_interface_id(
        cfg: UdpInterfaceConfig,
    ) -> Result<NetworkInterfaceId, InterfaceError> {
        let mut interfaces = INTERFACES.lock().unwrap(); // TODO wrap error
        let sock = SOCKETS
            .lock()
            .or(Err(InterfaceError::NotFound))?
            .pop()
            .ok_or(InterfaceError::NotFound)?;
        sock.set_nonblocking(true).unwrap();
        sock.connect(cfg.destination.as_str()).unwrap();
        let sock = LimitedUdpSocket {
            sock,
            _rate: cfg.rate,
        };
        interfaces.push(sock);
        let id = cfg.id;

        Ok(NetworkInterfaceId::from(id))
    }
}

use std::sync::{Arc, Mutex};
use std::{mem::size_of, net::UdpSocket};

use apex_rs_linux::partition::ApexLinuxPartition;

use log::{error, trace};
use network_partition::prelude::{
    CreateNetworkInterfaceId, DataRate, InterfaceError, NetworkInterfaceId,
    PlatformNetworkInterface, UdpInterfaceConfig, VirtualLinkId,
};
use once_cell::sync::Lazy;
use small_trace::gpio_trace;

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
        let interfaces = INTERFACES.lock().unwrap();
        let sock = interfaces
            .iter()
            .find(|i| i.id == id)
            .ok_or(InterfaceError::NotFound)?;
        match sock.sock.recv(buffer) {
            Ok(read) => {
                gpio_trace!(begin_network_receive, id.0 as u16);
                let vl_id_len = size_of::<VirtualLinkId>();
                let vl_id = &buffer[0..vl_id_len];
                let mut vl_id_buf = [0u8; size_of::<VirtualLinkId>()];
                vl_id_buf.copy_from_slice(vl_id);
                let vl_id = u32::from_be_bytes(vl_id_buf);
                let vl_id = VirtualLinkId::from_u32(vl_id);
                let msg = &buffer[vl_id_len..read];
                trace!("Received message from UDP socket for VL {vl_id}: {:?}", msg);
                gpio_trace!(end_network_receive, id.0 as u16);
                Ok((vl_id, msg))
            }
            Err(_) => Err(InterfaceError::NoData),
        }
    }

    fn platform_interface_send_unchecked(
        id: NetworkInterfaceId,
        vl: VirtualLinkId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError> {
        let interfaces = INTERFACES.lock().unwrap();
        let sock = interfaces
            .iter()
            .find(|i| i.id == id)
            .ok_or(InterfaceError::NotFound)?;
        let vlid = vl.into_inner().to_be_bytes();
        let udp_buf = [vlid.as_slice(), buffer].concat();
        gpio_trace!(begin_network_send, id.0 as u16);
        let res = sock.sock.send(&udp_buf);
        gpio_trace!(end_network_send, id.0 as u16);
        match res {
            Ok(trans) => {
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
    id: NetworkInterfaceId,
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
            id: cfg.id,
            sock,
            _rate: cfg.rate,
        };
        interfaces.push(sock);
        let id = cfg.id;

        Ok(id)
    }
}

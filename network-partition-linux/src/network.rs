use a653rs_linux::partition::ApexLinuxPartition;
use log::{error, trace};
use network_partition::error::*;
use network_partition::prelude::*;
use small_trace::small_trace;
use std::{mem::size_of, net::UdpSocket};

#[derive(Debug)]
pub struct UdpNetworkInterface<const MTU: usize>;

static mut INTERFACES: Vec<LimitedUdpSocket> = Vec::new();

impl<const MTU: usize> PlatformNetworkInterface for UdpNetworkInterface<MTU> {
    type Configuration = InterfaceConfig;

    fn platform_interface_receive_unchecked(
        id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<(VirtualLinkId, &'_ [u8]), InterfaceError> {
        let sock = get_interface(id)?;
        match sock.sock.recv(buffer) {
            Ok(read) => {
                small_trace!(begin_network_receive, id.0 as u16);
                let vl_id_len = size_of::<VirtualLinkId>();
                let vl_id = &buffer[0..vl_id_len];
                let mut vl_id_buf = [0u8; size_of::<VirtualLinkId>()];
                vl_id_buf.copy_from_slice(vl_id);
                let vl_id = u32::from_be_bytes(vl_id_buf);
                let vl_id = VirtualLinkId::from_u32(vl_id);
                let msg = &buffer[vl_id_len..read];
                trace!("Received message from UDP socket for VL {vl_id}: {:?}", msg);
                small_trace!(end_network_receive, id.0 as u16);
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
        // This is safe, because the interfaces are only created before the list of
        // interfaces is used
        let sock = get_interface(id)?;
        let vlid = vl.into_inner().to_be_bytes();
        let udp_buf = [vlid.as_slice(), buffer].concat();
        small_trace!(begin_network_send, id.0 as u16);
        let res = sock.sock.send(&udp_buf);
        small_trace!(end_network_send, id.0 as u16);
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

/// This is only safe, because the interfaces are only used *after* the list of
/// interfaces is created and the list of interfaces is never accessed
/// concurrently.
fn get_interface(id: NetworkInterfaceId) -> Result<&'static LimitedUdpSocket, InterfaceError> {
    unsafe {
        INTERFACES
            .get(id.0 as usize)
            .ok_or(InterfaceError::NotFound)
    }
}

// This is safe, because the interfaces are only created before the list of
// interfaces is used.
fn add_interface(s: LimitedUdpSocket) -> NetworkInterfaceId {
    unsafe {
        INTERFACES.push(s);
        NetworkInterfaceId(INTERFACES.len() as u32)
    }
}

#[derive(Debug)]
struct LimitedUdpSocket {
    sock: UdpSocket,
    _rate: DataRate,
}

fn get_socket(cfg: &InterfaceConfig) -> Result<UdpSocket, InterfaceError> {
    let res = ApexLinuxPartition::get_udp_socket(cfg.source.as_str());
    trace!("{:?}", cfg.source);
    res.ok().flatten().ok_or(InterfaceError::NotFound)
}

impl<const MTU: usize> CreateNetworkInterfaceId<UdpNetworkInterface<MTU>>
    for UdpNetworkInterface<MTU>
{
    fn create_network_interface_id(
        cfg: InterfaceConfig,
    ) -> Result<NetworkInterfaceId, InterfaceError> {
        let sock = get_socket(&cfg)?;
        sock.set_nonblocking(true).unwrap();
        sock.connect(cfg.destination.as_str()).unwrap();
        let sock = LimitedUdpSocket {
            sock,
            _rate: cfg.rate,
        };
        Ok(add_interface(sock))
    }
}

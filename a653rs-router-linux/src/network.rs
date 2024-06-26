use a653rs_linux::partition::ApexLinuxPartition;
use a653rs_router::prelude::*;
use std::net::UdpSocket;

#[derive(Debug)]
pub struct UdpNetworkInterface<const MTU: usize>;

static mut INTERFACES: Vec<LimitedUdpSocket> = Vec::new();

impl<const MTU: usize> PlatformNetworkInterface for UdpNetworkInterface<MTU> {
    fn platform_interface_receive_unchecked(
        id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<&'_ [u8], InterfaceError> {
        let sock = get_interface(id)?;
        match sock.sock.recv(buffer) {
            Ok(read) => {
                let msg = &buffer[..read];
                router_trace!("Received message from UDP socket");
                Ok(msg)
            }
            Err(_) => Err(InterfaceError::NoData),
        }
    }

    fn platform_interface_send_unchecked(
        id: NetworkInterfaceId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError> {
        // This is safe, because the interfaces are only created before the list of
        // interfaces is used
        let sock = get_interface(id)?;
        let res = sock.sock.send(buffer);
        match res {
            Ok(trans) => {
                router_trace!("Send {} bytes to UDP socket", buffer.len());
                Ok(trans)
            }
            Err(e) => {
                router_debug!("Failed to send to UDP socket: {:?}", e);
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
fn add_interface(s: LimitedUdpSocket) -> Result<NetworkInterfaceId, InterfaceError> {
    unsafe {
        let id = NetworkInterfaceId(INTERFACES.len() as u32);
        INTERFACES.push(s);
        Ok(id)
    }
}

#[derive(Debug)]
struct LimitedUdpSocket {
    sock: UdpSocket,
    _rate: DataRate,
}

fn get_socket(cfg: &InterfaceConfig) -> Result<UdpSocket, InterfaceError> {
    let res = ApexLinuxPartition::get_udp_socket(cfg.source.as_str());
    router_debug!("{:?}", cfg.source);
    res.ok().flatten().ok_or(InterfaceError::NotFound)
}

impl<const MTU: usize> CreateNetworkInterfaceId<UdpNetworkInterface<MTU>>
    for UdpNetworkInterface<MTU>
{
    fn create_network_interface_id(
        cfg: &InterfaceConfig,
    ) -> Result<NetworkInterfaceId, InterfaceError> {
        let sock = get_socket(cfg)?;
        sock.set_nonblocking(true)
            .or(Err(InterfaceError::SendFailed))?;
        sock.connect(cfg.destination.as_str())
            .or(Err(InterfaceError::SendFailed))?;
        let sock = LimitedUdpSocket {
            sock,
            _rate: cfg.rate,
        };
        add_interface(sock)
    }
}

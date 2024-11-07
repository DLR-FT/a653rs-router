use a653rs::{bindings::*, prelude::*};
use core::{fmt::Display, time::Duration};

use crate::{
    network::PayloadSize,
    router::{RouterInput, RouterOutput},
};

impl<S: ApexSamplingPortP4> RouterInput for SamplingPortDestination<S> {
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], PortError> {
        router_bench!(begin_apex_receive, self.id() as u16);
        let res = self.receive(buf);
        router_bench!(end_apex_receive, self.id() as u16);
        let (_val, data) = res.map_err(|_e| PortError::Receive)?;
        Ok(data)
    }

    fn mtu(&self) -> PayloadSize {
        self.size() as PayloadSize
    }
}

impl<S: ApexSamplingPortP4> RouterOutput for SamplingPortSource<S> {
    fn send(&self, buf: &[u8]) -> Result<(), PortError> {
        router_bench!(begin_apex_send, self.id() as u16);
        let res = self.send(buf);
        router_bench!(end_apex_send, self.id() as u16);
        res.map_err(|_e| PortError::Send)?;
        Ok(())
    }

    fn mtu(&self) -> PayloadSize {
        self.size() as PayloadSize
    }
}

impl<Q: ApexQueuingPortP4> RouterInput for QueuingPortReceiver<Q> {
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], PortError> {
        const TIMEOUT: SystemTime = SystemTime::Normal(Duration::ZERO);
        router_bench!(begin_apex_send, self.id() as u16);
        let res = self.receive(buf, TIMEOUT);
        router_bench!(end_apex_send, self.id() as u16);
        let (buf, _overflow) = res.map_err(|e| match e {
            Error::NotAvailable => PortError::WouldBlock,
            _ => PortError::Receive,
        })?;
        Ok(buf)
    }

    fn mtu(&self) -> PayloadSize {
        self.size()
    }
}

impl<Q: ApexQueuingPortP4> RouterOutput for QueuingPortSender<Q> {
    fn send(&self, buf: &[u8]) -> Result<(), PortError> {
        const TIMEOUT: SystemTime = SystemTime::Normal(Duration::ZERO);
        router_bench!(begin_apex_send, self.id() as u16);
        let res = self.send(buf, TIMEOUT);
        router_bench!(end_apex_send, self.id() as u16);
        res.map_err(|_e| PortError::Send)?;
        Ok(())
    }

    fn mtu(&self) -> PayloadSize {
        self.size()
    }
}

/// An error occured while reading or writing a port of the router.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortError {
    /// Operation would block
    WouldBlock,

    /// Failed to send from router output
    Send,

    /// Failed to receive from router input
    Receive,

    /// Creating of the router input / output failed
    Create,
}

impl Display for PortError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self {
            Self::Send => write!(f, "Failed to send from router output"),
            Self::Receive => write!(f, "Failed to receive from router input"),
            Self::Create => write!(f, "Failed to create router port"),
            Self::WouldBlock => write!(f, "Operation would block"),
        }
    }
}

use a653rs::{bindings::*, prelude::*};
use core::time::Duration;

use crate::prelude::*;

impl<const M: MessageSize, S: ApexSamplingPortP4> RouterInput for SamplingPortDestination<M, S> {
    fn receive<'a>(
        &self,
        vl: &VirtualLinkId,
        buf: &'a mut [u8],
    ) -> Result<(VirtualLinkId, &'a [u8]), PortError> {
        router_bench!(begin_apex_receive, vl.0 as u16);
        let res = self.receive(buf);
        router_bench!(end_apex_receive, vl.0 as u16);
        let (_val, data) = res.map_err(|_e| PortError::Receive)?;
        Ok((*vl, data))
    }
}

impl<const M: MessageSize, S: ApexSamplingPortP4> RouterOutput for SamplingPortSource<M, S> {
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), PortError> {
        router_bench!(begin_apex_send, vl.0 as u16);
        let res = self.send(buf);
        router_bench!(end_apex_send, vl.0 as u16);
        res.map_err(|_e| PortError::Send)?;
        Ok(())
    }
}

impl<const M: MessageSize, const R: MessageRange, Q: ApexQueuingPortP4> RouterInput
    for QueuingPortReceiver<M, R, Q>
{
    fn receive<'a>(
        &self,
        vl: &VirtualLinkId,
        buf: &'a mut [u8],
    ) -> Result<(VirtualLinkId, &'a [u8]), PortError> {
        let timeout = SystemTime::Normal(Duration::from_micros(10));
        router_bench!(begin_apex_send, vl.0 as u16);
        let res = self.receive(buf, timeout);
        router_bench!(end_apex_send, vl.0 as u16);
        let (buf, _overflow) = res.map_err(|_e| PortError::Receive)?;
        Ok((*vl, buf))
    }
}

impl<const M: MessageSize, const R: MessageRange, Q: ApexQueuingPortP4> RouterOutput
    for QueuingPortSender<M, R, Q>
{
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), PortError> {
        let timeout = SystemTime::Normal(Duration::from_micros(10));
        router_bench!(begin_apex_send, vl.0 as u16);
        let res = self.send(buf, timeout);
        router_bench!(end_apex_send, vl.0 as u16);
        res.map_err(|_e| PortError::Send)?;
        Ok(())
    }
}

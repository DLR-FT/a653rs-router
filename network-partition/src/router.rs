//! Router

use crate::{
    error::Error,
    scheduler::{Scheduler, TimeSource},
    types::VirtualLinkId,
};
use a653rs::prelude::{
    ApexQueuingPortP4, ApexSamplingPortP4, MessageRange, MessageSize, QueuingPortReceiver,
    QueuingPortSender, SamplingPortDestination, SamplingPortSource, SystemTime, Validity,
};
use core::{fmt::Debug, time::Duration};
use heapless::{LinearMap, Vec};
use log::{trace, warn};
use small_trace::small_trace;

/// An input to a virtual link.
pub trait RouterInput {
    /// Receives a message and store it into `buf`.
    ///
    /// The returned `VirtualLinkId` indicates for which virtual link the
    /// received data is destined. Usually, this will be the same virtual
    /// link as has been specified by `VirtualLinkId`, but when the `Input`
    /// multiplexes multiple virtual links, the next received message may be
    /// (unexpectedly) for another virtual link. Implementations may choose to
    /// indicate this or return an error, if they should do not receive
    /// multiple virtual links.
    fn receive<'a>(
        &self,
        vl: &VirtualLinkId,
        buf: &'a mut [u8],
    ) -> Result<(VirtualLinkId, &'a [u8]), Error>;
}

/// An output from a virtual link.
pub trait RouterOutput {
    /// Sends `buf` to a virtual link on this `Output`.
    ///
    /// Returns a slice to the portion of `buf` that has *not* been transmitted.
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), Error>;
}

type RouteTable<'a, const I: usize, const O: usize> =
    LinearMap<VirtualLinkId, Vec<&'a dyn RouterOutput, O>, I>;

type Inputs<'a, const I: usize> = LinearMap<VirtualLinkId, &'a dyn RouterInput, I>;

/// The router containing the routing information.
#[derive(Clone)]
pub struct Router<'a, const I: usize, const O: usize> {
    inputs: Inputs<'a, I>,
    outputs: RouteTable<'a, I, O>,
}

impl<'a, const I: usize, const O: usize> Router<'a, I, O> {
    /// Forwards a virtual link from its source to its destinations.
    fn route<const B: usize>(&self, vl: &VirtualLinkId) -> Result<(), Error> {
        let buf = &mut [0u8; B];
        let input = self.inputs.get(vl).ok_or(Error::InvalidConfig)?;
        // This vl may be different, than the one specified.
        let (vl, buf) = input.receive(vl, buf)?;
        let outs = self.outputs.get(&vl).ok_or(Error::InvalidConfig)?;
        for out in outs.into_iter() {
            out.send(&vl, buf)?;
        }
        Ok(())
    }

    /// Forwards messages between the hypervisor and the network.
    pub fn forward<const B: usize>(
        &self,
        scheduler: &mut dyn Scheduler,
        time_source: &dyn TimeSource,
    ) -> Result<Option<VirtualLinkId>, Error> {
        let time = time_source.get_time()?;
        if let Some(next) = scheduler.schedule_next(&time) {
            small_trace!(begin_virtual_link_scheduled, next.0 as u16);
            trace!("Scheduled VL {next}");
            let res = self.route::<B>(&next);
            small_trace!(end_virtual_link_scheduled, next.0 as u16);
            res.map(|_| Some(next))
        } else {
            trace!("Scheduled no VL");
            Ok(None)
        }
    }
}

/// Creates a new builder for a router.
pub fn builder<'a, const I: usize, const O: usize>() -> RouterBuilder<'a, I, O> {
    Default::default()
}

impl<'a, const I: usize, const O: usize> Debug for Router<'a, I, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Router")
    }
}

type Routes<'a, const I: usize, const O: usize> =
    LinearMap<VirtualLinkId, (&'a dyn RouterInput, Vec<&'a dyn RouterOutput, O>), I>;

/// Builds a new router.
#[derive(Default)]
pub struct RouterBuilder<'a, const I: usize, const O: usize> {
    vls: Routes<'a, I, O>,
}

impl<'a, const I: usize, const O: usize> Debug for RouterBuilder<'a, I, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("RouterBuilder")
    }
}

impl<'a, const I: usize, const O: usize> RouterBuilder<'a, I, O> {
    pub fn route(
        &mut self,
        vl: &VirtualLinkId,
        input: &'a dyn RouterInput,
        output: &Vec<&'a dyn RouterOutput, O>,
    ) -> Result<&mut Self, Error> {
        if self.vls.contains_key(vl) {
            return Err(Error::InvalidConfig);
        }
        _ = self
            .vls
            .insert(*vl, (input, output.clone()))
            .map_err(|_e| Error::InvalidConfig)?;
        Ok(self)
    }

    pub fn build(&self) -> Result<Router<'a, I, O>, Error> {
        let mut inputs = Inputs::default();
        let mut outputs = RouteTable::default();
        for (id, (i, o)) in self.vls.iter() {
            if inputs.contains_key(id) || outputs.contains_key(id) {
                return Err(Error::InvalidConfig);
            }
            _ = inputs
                .insert(*id, (*i).clone())
                .or(Err(Error::InvalidConfig));
            _ = outputs.insert(*id, o.clone()).or(Err(Error::InvalidConfig));
        }
        Ok(Router::<'a, I, O> { inputs, outputs })
    }
}

impl<const M: MessageSize, S: ApexSamplingPortP4> RouterInput for SamplingPortDestination<M, S> {
    fn receive<'a>(
        &self,
        vl: &VirtualLinkId,
        buf: &'a mut [u8],
    ) -> Result<(VirtualLinkId, &'a [u8]), Error> {
        if buf.len() > M as usize {
            return Err(Error::InvalidConfig);
        }
        small_trace!(begin_apex_receive, vl.0 as u16);
        let res = self.receive(buf);
        small_trace!(end_apex_receive, vl.0 as u16);
        match res {
            Ok((val, data)) => {
                if val == Validity::Invalid {
                    warn!("Reading invalid data from port");
                    Err(Error::InvalidData)
                } else {
                    Ok((*vl, data))
                }
            }
            Err(_e) => {
                warn!("Failed to receive data from port");
                Err(Error::PortReceiveFail)
            }
        }
    }
}

impl<const M: MessageSize, S: ApexSamplingPortP4> RouterOutput for SamplingPortSource<M, S> {
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), Error> {
        if buf.len() < M as usize {
            return Err(Error::InvalidConfig);
        }
        // TODO small_trace!(begin_forward_to_apex, vl.0 as u16);
        small_trace!(begin_apex_send, vl.0 as u16);
        let res = self.send(buf);
        small_trace!(end_apex_send, vl.0 as u16);
        if let Err(_e) = res {
            warn!("Failed to write to sampling port");
            Err(Error::PortSendFail)
        } else {
            trace!("Wrote to source: {buf:?}");
            Ok(())
        }
    }
}

impl<const M: MessageSize, const R: MessageRange, Q: ApexQueuingPortP4> RouterInput
    for QueuingPortReceiver<M, R, Q>
{
    fn receive<'a>(
        &self,
        vl: &VirtualLinkId,
        buf: &'a mut [u8],
    ) -> Result<(VirtualLinkId, &'a [u8]), Error> {
        if buf.len() < M as usize {
            return Err(Error::InvalidConfig);
        }
        let timeout = SystemTime::Normal(Duration::from_micros(10));
        small_trace!(begin_apex_send, vl.0 as u16);
        let res = self.receive(buf, timeout);
        small_trace!(end_apex_send, vl.0 as u16);
        match res {
            Err(_e) => {
                warn!("Failed to write to queuing port");
                Err(Error::PortSendFail)
            }
            Ok(buf) => {
                trace!("Received data from local queueing port");
                if buf.is_empty() {
                    warn!("Dropping empty message");
                    Err(Error::InvalidData)
                } else {
                    Ok((*vl, buf))
                }
            }
        }
    }
}

impl<const M: MessageSize, const R: MessageRange, Q: ApexQueuingPortP4> RouterOutput
    for QueuingPortSender<M, R, Q>
{
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), Error> {
        if buf.len() > M as usize {
            return Err(Error::InvalidConfig);
        }
        let timeout = SystemTime::Normal(Duration::from_micros(10));
        small_trace!(begin_apex_send, vl.0 as u16);
        let res = self.send(buf, timeout);
        small_trace!(end_apex_send, vl.0 as u16);
        if let Err(_e) = res {
            warn!("Failed to write to queuing port");
            Err(Error::PortSendFail)
        } else {
            trace!("Wrote to source: {buf:?}");
            Ok(())
        }
    }
}

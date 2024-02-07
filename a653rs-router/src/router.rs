//! Router

use crate::{
    error::Error,
    reconfigure::CfgError,
    scheduler::{ScheduleError, Scheduler, TimeSource},
    types::VirtualLinkId,
};

use core::fmt::{Debug, Display};
use heapless::{LinearMap, Vec};

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
    ) -> Result<(VirtualLinkId, &'a [u8]), PortError>;
}

/// An output from a virtual link.
pub trait RouterOutput {
    /// Sends `buf` to a virtual link on this `Output`.
    ///
    /// Returns a slice to the portion of `buf` that has *not* been transmitted.
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), PortError>;
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
        let input = self.inputs.get(vl).ok_or(RouteError::InvalidVl)?;
        // This vl may be different, than the one specified.
        let (vl, buf) = input.receive(vl, buf)?;
        router_trace!("Received from {vl:?}: {buf:?}");
        let outs = self.outputs.get(&vl).ok_or(RouteError::InvalidVl)?;
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
        let time = time_source.get_time().map_err(ScheduleError::from)?;
        if let Some(next) = scheduler.schedule_next(&time) {
            router_bench!(begin_virtual_link_scheduled, next.0 as u16);
            let res = self.route::<B>(&next);
            router_bench!(end_virtual_link_scheduled, next.0 as u16);
            res.map(|_| Some(next))
        } else {
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
    ) -> Result<&mut Self, CfgError> {
        if self.vls.contains_key(vl) {
            return Err(CfgError::InvalidVl);
        }
        _ = self
            .vls
            .insert(*vl, (input, output.clone()))
            .map_err(|_e| CfgError::Storage)?;
        Ok(self)
    }

    pub fn build(&self) -> Result<Router<'a, I, O>, CfgError> {
        let mut inputs = Inputs::default();
        let mut outputs = RouteTable::default();
        for (id, (i, o)) in self.vls.iter() {
            if inputs.contains_key(id) {
                return Err(CfgError::InvalidInput);
            }
            if outputs.contains_key(id) {
                return Err(CfgError::InvalidOutput);
            }
            _ = inputs.insert(*id, *i).or(Err(CfgError::Storage));
            _ = outputs.insert(*id, o.clone()).or(Err(CfgError::Storage));
        }
        Ok(Router::<'a, I, O> { inputs, outputs })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteError {
    InvalidVl,
}

/// An error occured while reading or writing a port of the router.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortError {
    /// Failed to send from router output
    Send,

    /// Failed to receive from router input
    Receive,
}

impl Display for PortError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self {
            Self::Send => write!(f, "Failed to send from router output"),
            Self::Receive => write!(f, "Failed to receive from router input"),
        }
    }
}

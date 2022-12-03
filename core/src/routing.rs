//! Routing

use crate::error::Error;
use crate::ports::ChannelId;
use crate::prelude::VirtualLinkId;
use heapless::Vec;

type RouteTableEntry<S, D> = (S, D);

// TODO use heapless::LinearMap
#[derive(Debug, Default)]
struct RouteTable<const TABLE_SIZE: usize> {
    input: Vec<RouteTableEntry<VirtualLinkId, ChannelId>, TABLE_SIZE>,
    output: Vec<RouteTableEntry<ChannelId, VirtualLinkId>, TABLE_SIZE>,
}

impl<const TABLE_SIZE: usize> RouteTable<TABLE_SIZE> {
    fn get_local_destinations<'a>(
        &'a self,
        source: &'a VirtualLinkId,
    ) -> Result<impl Iterator<Item = ChannelId> + 'a, Error> {
        let is_empty = self.input.iter().find(|x| x.0 == *source).is_none();
        if is_empty {
            return Err(Error::NoRoute);
        }
        let destinations = self.input.iter().filter_map(|x| {
            if x.0 == *source {
                Some(x.1.clone())
            } else {
                None
            }
        });
        Ok(destinations)
    }

    fn get_remote_destinations<'a>(
        &'a self,
        source: &'a ChannelId,
    ) -> Result<VirtualLinkId, Error> {
        let destination = self
            .output
            .iter()
            .find(|x| x.0 == *source)
            .ok_or(Error::NoRoute)?;
        Ok(destination.1)
    }

    fn add_output_route(
        &mut self,
        source: ChannelId,
        destination: VirtualLinkId,
    ) -> Result<(), Error> {
        if let Some(_) = self.output.iter().find(|&x| x.1 == destination) {
            return Err(Error::InvalidRoute);
        }
        if let Err(_) = self.output.push((source, destination)) {
            return Err(Error::InvalidRoute);
        }
        Ok(())
    }

    fn add_input_route(
        &mut self,
        source: VirtualLinkId,
        destination: ChannelId,
    ) -> Result<(), Error> {
        if let Some(_) = self.input.iter().find(|&x| x.1 == destination) {
            return Err(Error::InvalidRoute);
        }
        if let Err(_) = self.input.push((source, destination)) {
            return Err(Error::InvalidRoute);
        }
        Ok(())
    }
}

/// A router that forwards messages.
///
/// The router can forward messages explicitly from one address to another.
/// This is meant for special services like the an echo service that locally forwards messages directly from one
/// port to another on the same hypervisor.
/// The router forwards messages either according to the rules of an input route table (remote address -> local address)
/// or according to an output route table (local address -> remote address).
#[derive(Debug)]
pub struct Router<const TABLE_SIZE: usize> {
    route_table: RouteTable<TABLE_SIZE>,
}

impl<const TABLE_SIZE: usize> Router<TABLE_SIZE> {
    /// Creates a new router.
    pub fn new() -> Self {
        Router::<TABLE_SIZE> {
            route_table: RouteTable::default(),
        }
    }

    /// Route remote input from network to local ports.
    pub fn route_remote_input<'a>(
        &'a self,
        source: &'a VirtualLinkId,
    ) -> Result<impl Iterator<Item = ChannelId> + 'a, Error> {
        let destinations = self.route_table.get_local_destinations(source)?;
        Ok(destinations)
    }

    /// Route local output from ports to network.
    pub fn route_local_output(&self, source: &ChannelId) -> Result<VirtualLinkId, Error> {
        let destinations = self.route_table.get_remote_destinations(source)?;
        Ok(destinations)
    }

    /// Add an output route.
    pub fn add_output_route(
        &mut self,
        source: ChannelId,
        destination: VirtualLinkId,
    ) -> Result<(), Error> {
        self.route_table.add_output_route(source, destination)
    }

    /// Add an input route.
    pub fn add_input_route(
        &mut self,
        source: VirtualLinkId,
        destination: ChannelId,
    ) -> Result<(), Error> {
        self.route_table.add_input_route(source, destination)
    }
}

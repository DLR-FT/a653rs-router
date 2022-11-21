//! Routing

use crate::error::Error;
use crate::ports::{ChannelId, VirtualLinkId};
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
    ) -> Result<Vec<ChannelId, TABLE_SIZE>, Error> {
        let is_empty = self.input.iter().find(|x| x.0 == *source).is_none();
        if is_empty {
            return Err(Error::NoRoute);
        }
        let destinations: Vec<ChannelId, TABLE_SIZE> = self
            .input
            .iter()
            .filter_map(|x| {
                if x.0 == *source {
                    Some(x.1.clone())
                } else {
                    None
                }
            })
            .collect();
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
pub trait Router {
    /// An address from the set of local addresses (e.g. channel names / port ids).
    type LocalAddress;

    /// An address from the set of remote addresses (e.g. virtual link IDs).
    type RemoteAddress;

    /// A list of local addresses.
    type LocalAddressList;

    /// Forward an incoming message to a local port.
    fn route_remote_input(
        &self,
        source: &Self::RemoteAddress,
    ) -> Result<Self::LocalAddressList, Error>;

    /// Forward an outgoing message to an outgoing link.
    fn route_local_output(&self, source: &Self::LocalAddress)
        -> Result<Self::RemoteAddress, Error>;

    /// Add an output route.
    fn add_output_route(
        &mut self,
        source: Self::LocalAddress,
        destination: Self::RemoteAddress,
    ) -> Result<(), Error>;

    /// Add an input route.
    fn add_input_route(
        &mut self,
        source: Self::RemoteAddress,
        destination: Self::LocalAddress,
    ) -> Result<(), Error>;
}

/// A router that uses the P4 interface of the hypervisor.
///
/// The router holds references to all local ports, because only P1 would support looking up channels from the hypervisor.
#[derive(Debug)]
pub struct RouterP4<const TABLE_SIZE: usize> {
    route_table: RouteTable<TABLE_SIZE>,
}

impl<const TABLE_SIZE: usize> RouterP4<TABLE_SIZE> {
    /// Creates a new router.
    pub fn new() -> Self {
        RouterP4::<TABLE_SIZE> {
            route_table: RouteTable::default(),
        }
    }
}

impl<const TABLE_SIZE: usize> Router for RouterP4<TABLE_SIZE> {
    // TODO refactor result handling and port reading / sending
    type LocalAddress = ChannelId;
    type RemoteAddress = VirtualLinkId;
    type LocalAddressList = Vec<ChannelId, TABLE_SIZE>;

    fn route_remote_input<'a>(
        &self,
        source: &'a Self::RemoteAddress,
    ) -> Result<Self::LocalAddressList, Error> {
        let destinations = self.route_table.get_local_destinations(source)?;
        Ok(destinations)
    }

    fn route_local_output(
        &self,
        source: &Self::LocalAddress,
    ) -> Result<Self::RemoteAddress, Error> {
        let destinations = self.route_table.get_remote_destinations(source)?;
        Ok(destinations)
    }

    /// Add an output route.
    fn add_output_route(
        &mut self,
        source: Self::LocalAddress,
        destination: Self::RemoteAddress,
    ) -> Result<(), Error> {
        self.route_table.add_output_route(source, destination)
    }

    /// Add an input route.
    fn add_input_route(
        &mut self,
        source: Self::RemoteAddress,
        destination: Self::LocalAddress,
    ) -> Result<(), Error> {
        self.route_table.add_input_route(source, destination)
    }
}

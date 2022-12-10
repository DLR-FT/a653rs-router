//! Routing

use crate::error::{Error, RouteError};
use crate::ports::PortId;
use crate::prelude::VirtualLinkId;
use heapless::Vec;

type RouteTableEntry<S, D> = (S, D);

// TODO use heapless::LinearMap
#[derive(Debug, Default)]
struct RouteTable<const TABLE_SIZE: usize> {
    input: Vec<RouteTableEntry<VirtualLinkId, PortId>, TABLE_SIZE>,
    output: Vec<RouteTableEntry<PortId, VirtualLinkId>, TABLE_SIZE>,
}

impl<const TABLE_SIZE: usize> RouteTable<TABLE_SIZE> {
    fn get_local_destinations<'a>(
        &'a self,
        source: &'a VirtualLinkId,
    ) -> Result<PortIdIterator<TABLE_SIZE>, Error> {
        if !self.input.iter().any(|x| x.0 == *source) {
            return Err(Error::NoRoute);
        }
        let destinations =
            self.input
                .iter()
                .filter_map(|x| if x.0 == *source { Some(x.1) } else { None });
        Ok(destinations.collect())
    }

    fn get_remote_destinations<'a>(&'a self, source: &'a PortId) -> Result<VirtualLinkId, Error> {
        let destination = self
            .output
            .iter()
            .find(|x| x.0 == *source)
            .ok_or(Error::NoRoute)?;
        Ok(destination.1)
    }

    fn add_output_route(
        &mut self,
        source: PortId,
        destination: VirtualLinkId,
    ) -> Result<(), Error> {
        if self.output.iter().any(|x| x.1 == destination) {
            return Err(Error::InvalidRoute(RouteError::from(source)));
        }
        if self.output.push((source, destination)).is_err() {
            return Err(Error::InvalidRoute(RouteError::from(source)));
        }
        Ok(())
    }

    fn add_input_route(&mut self, source: VirtualLinkId, destination: PortId) -> Result<(), Error> {
        if self.input.iter().any(|x| x.1 == destination) {
            return Err(Error::InvalidRoute(RouteError::from(source)));
        }
        if self.input.push((source, destination)).is_err() {
            return Err(Error::InvalidRoute(RouteError::from(source)));
        }
        Ok(())
    }
}

/// Performs route lookups.
/// `PORTS` defines the maximum number of ports that can be returned.
pub trait RouteLookup<const PORTS: usize> {
    /// Route remote input from network to local ports.
    fn route_remote_input<'a>(
        &'a self,
        source: &'a VirtualLinkId,
    ) -> Result<PortIdIterator<PORTS>, Error>;

    /// Route local output from ports to network.
    fn route_local_output(&self, source: &PortId) -> Result<VirtualLinkId, Error>;
}

/// A router that forwards messages.
///
/// The router can forward messages explicitly from one address to another.
/// This is meant for special services like the an echo service that locally forwards messages directly from one
/// port to another on the same hypervisor.
/// The router forwards messages either according to the rules of an input route table (remote address -> local address)
/// or according to an output route table (local address -> remote address).
#[derive(Default, Debug)]
pub struct Router<const TABLE_SIZE: usize> {
    route_table: RouteTable<TABLE_SIZE>,
}

impl<const TABLE_SIZE: usize> Router<TABLE_SIZE> {
    /// Add an output route.
    pub fn add_output_route(
        &mut self,
        source: PortId,
        destination: VirtualLinkId,
    ) -> Result<(), Error> {
        self.route_table.add_output_route(source, destination)
    }

    /// Add an input route.
    pub fn add_input_route(
        &mut self,
        source: VirtualLinkId,
        destination: PortId,
    ) -> Result<(), Error> {
        self.route_table.add_input_route(source, destination)
    }
}

impl<const PORTS: usize> RouteLookup<PORTS> for Router<PORTS> {
    fn route_remote_input<'a>(
        &'a self,
        source: &'a VirtualLinkId,
    ) -> Result<PortIdIterator<PORTS>, Error> {
        let destinations = self.route_table.get_local_destinations(source)?;
        let ports = destinations;
        Ok(ports)
    }

    fn route_local_output(&self, source: &PortId) -> Result<VirtualLinkId, Error> {
        let destinations = self.route_table.get_remote_destinations(source)?;
        Ok(destinations)
    }
}

/// An iterator over the IDs of destination ports.
#[derive(Debug)]
pub struct PortIdIterator<const TABLE_SIZE: usize> {
    ports: Vec<PortId, TABLE_SIZE>,
    current: usize,
}

impl<const TABLE_SIZE: usize> FromIterator<PortId> for PortIdIterator<TABLE_SIZE> {
    fn from_iter<T: IntoIterator<Item = PortId>>(iter: T) -> Self {
        let mut p = Self {
            ports: Vec::default(),
            current: 0,
        };
        let at_most = iter.into_iter().take(TABLE_SIZE);
        for n in at_most {
            p.ports.push(n).unwrap();
        }
        p
    }
}

impl<const TABLE_SIZE: usize> Iterator for PortIdIterator<TABLE_SIZE> {
    type Item = PortId;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.current;
        if next < self.ports.len() {
            self.current += 1;
            Some(self.ports[next])
        } else {
            None
        }
    }
}

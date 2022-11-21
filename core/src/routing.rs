//! Routing

use crate::error::Error;
use crate::ports::{ChannelName, VirtualLinkId};
use apex_rs::prelude::{
    ApexSamplingPortP4, MessageSize, SamplingPortDestination, SamplingPortSource, Validity,
};

type RouteTableEntry<S, D> = (S, D);

trait RouteLookup<L, R> {
    type RemoteDestinations: Iterator<Item = R>;
    type LocalDestinations: Iterator<Item = L>;

    fn get_local_destinations(&self, source: &R) -> Result<Self::LocalDestinations, Error>;

    fn get_remote_destinations(&self, source: &L) -> Result<Self::RemoteDestinations, Error>;
}

#[derive(Debug, Default)]
struct RouteTable<L, R> {
    input: Vec<RouteTableEntry<R, L>>,
    output: Vec<RouteTableEntry<L, R>>,
}

impl<L, R> RouteLookup<L, R> for RouteTable<L, R>
where
    L: PartialEq + Clone,
    R: PartialEq + Clone,
{
    type RemoteDestinations = std::vec::IntoIter<R>;
    type LocalDestinations = std::vec::IntoIter<L>;

    fn get_local_destinations(&self, source: &R) -> Result<Self::LocalDestinations, Error> {
        let destinations: Vec<L> = self
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
        if destinations.is_empty() {
            Err(Error::NoRoute)
        } else {
            Ok(destinations.into_iter())
        }
    }

    fn get_remote_destinations(&self, source: &L) -> Result<Self::RemoteDestinations, Error> {
        let destinations: Vec<R> = self
            .output
            .iter()
            .filter_map(|x| {
                if x.0 == *source {
                    Some(x.1.clone())
                } else {
                    None
                }
            })
            .collect();
        if destinations.is_empty() {
            Err(Error::NoRoute)
        } else {
            Ok(destinations.into_iter())
        }
    }
}

trait RouteModify<L, R> {
    type RemoteDestinations: Iterator<Item = R>;
    type LocalDestinations: Iterator<Item = L>;

    fn add_output_route(&mut self, source: L, destination: R) -> Result<(), Error>;
    fn add_input_route(&mut self, source: R, destination: L) -> Result<(), Error>;
}

impl<L, R> RouteModify<L, R> for RouteTable<L, R>
where
    L: PartialEq,
    R: PartialEq,
{
    type RemoteDestinations = std::vec::IntoIter<R>;
    type LocalDestinations = std::vec::IntoIter<L>;

    // TODO refactor
    fn add_output_route(&mut self, source: L, destination: R) -> Result<(), Error> {
        if let Some(_) = self.output.iter().find(|&x| x.1 == destination) {
            return Err(Error::InvalidRoute);
        }
        self.output.push((source, destination));
        Ok(())
    }

    fn add_input_route(&mut self, source: R, destination: L) -> Result<(), Error> {
        if let Some(_) = self.input.iter().find(|&x| x.1 == destination) {
            return Err(Error::InvalidRoute);
        }
        self.input.push((source, destination));
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

    /// TODO replace echo with port -> VL -> port on same hypervisor. This needs a loopback interface / VL that connects ports on same hypervisor.
    fn echo<const MSG_SIZE: MessageSize>(
        &self,
        source: &Self::LocalAddress,
        destination: &Self::LocalAddress,
    ) -> Result<(), Error>
    where
        [u8; MSG_SIZE as usize]:;

    /// Forward an incoming message to a local port.
    fn route_remote_input<const MSG_SIZE: MessageSize>(
        &self,
        source: &Self::RemoteAddress,
    ) -> Result<(), Error>
    where
        [u8; MSG_SIZE as usize]:;

    /// Forward an outgoing message to an outgoing link.
    fn route_local_output<const MSG_SIZE: MessageSize>(
        &self,
        source: &Self::LocalAddress,
    ) -> Result<(), Error>
    where
        [u8; MSG_SIZE as usize]:;

    // fn add_output_route(
    //     &mut self,
    //     source: Self::LocalAddress,
    //     destination: Self::RemoteAddress,
    // ) -> Result<(), Error>;

    // fn add_input_route(
    //     &mut self,
    //     source: Self::RemoteAddress,
    //     destination: Self::LocalAddress,
    // ) -> Result<(), Error>;
}

/// A router that uses the P4 interface of the hypervisor.
///
/// The router holds references to all local ports, because only P1 would support looking up channels from the hypervisor.
#[derive(Debug)]
pub struct RouterP4<const MSG_SIZE: MessageSize, H>
where
    H: ApexSamplingPortP4,
{
    route_table: RouteTable<ChannelName, VirtualLinkId>,
    port_destinations: Vec<(ChannelName, SamplingPortDestination<MSG_SIZE, H>)>,
    port_sources: Vec<(ChannelName, SamplingPortSource<MSG_SIZE, H>)>,
}

impl<const MSG_SIZE: MessageSize, H> RouterP4<MSG_SIZE, H>
where
    H: ApexSamplingPortP4,
{
    /// Creare a new router with an empty route table.
    pub fn new() -> Self {
        RouterP4 {
            route_table: RouteTable::default(),
            port_destinations: vec![],
            port_sources: vec![],
        }
    }

    /// A a new port that can be used as a destination.
    pub fn add_local_destination(
        &mut self,
        channel: ChannelName,
        destination: SamplingPortDestination<MSG_SIZE, H>,
    ) {
        // TODO prevent addition of duplicate keys
        self.port_destinations.push((channel, destination))
    }

    /// A a new port that can be used as a source.
    pub fn add_local_source(
        &mut self,
        channel: ChannelName,
        destination: SamplingPortSource<MSG_SIZE, H>,
    ) {
        // TODO prevent addition of duplicate keys
        self.port_sources.push((channel, destination))
    }

    // TODO add_remote_source, add_remote_destination
}

impl<const SAMPLING_PORT_SIZE: MessageSize, H> Router for RouterP4<SAMPLING_PORT_SIZE, H>
where
    H: ApexSamplingPortP4,
{
    type LocalAddress = ChannelName;
    type RemoteAddress = VirtualLinkId;

    fn echo<const MSG_SIZE: MessageSize>(
        &self,
        source: &Self::LocalAddress,
        destination: &Self::LocalAddress,
    ) -> Result<(), Error>
    where
        [u8; MSG_SIZE as usize]:,
    {
        let in_port = self.port_destinations.iter().find(|x| x.0 == *source);

        let mut buffer = [0u8; MSG_SIZE as usize];
        let (validity, received) = match in_port {
            Some((_, port)) => port.receive(&mut buffer),
            None => Err(Error::ReceiveFail)?,
        }?;
        if validity == Validity::Invalid {
            return Err(Error::InvalidData);
        }
        if let Some((_, port)) = self.port_sources.iter().find(|x| x.0 == *destination) {
            let result = port.send(received)?;
            Ok(result)
        } else {
            Err(Error::NoLink)
        }
    }

    fn route_remote_input<const MSG_SIZE: MessageSize>(
        &self,
        source: &Self::RemoteAddress,
    ) -> Result<(), Error> {
        self.route_table
            .get_local_destinations(source)?
            .for_each(|_| todo!("Send to local")); // TODO report what failed
        Ok(())
    }

    fn route_local_output<const MSG_SIZE: MessageSize>(
        &self,
        source: &Self::LocalAddress,
    ) -> Result<(), Error> {
        self.route_table
            .get_remote_destinations(source)?
            .for_each(|_| todo!("Send to remote")); // TODO report what failed
        Ok(())
    }
}

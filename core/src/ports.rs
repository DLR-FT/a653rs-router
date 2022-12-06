use crate::error::Error;
use crate::prelude::PayloadSize;
use apex_rs::prelude::{
    ApexSamplingPortP4, MessageSize, SamplingPortDestination, SamplingPortSource, Validity,
};
use heapless::LinearMap;
use serde::{Deserialize, Serialize};

// TODO
// - iterate over self.ports
// - lookup virtual link id of port
// - read port contents into frame
// - return iterator over new frames<'a>

/// An ID of a hypervisor port.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct ChannelId(u32);

impl ChannelId {
    /// Returns the name of the channel.
    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl From<u32> for ChannelId {
    fn from(val: u32) -> ChannelId {
        ChannelId(val)
    }
}

impl From<ChannelId> for u32 {
    fn from(val: ChannelId) -> u32 {
        val.0
    }
}

/// A message from the hypervisor, but annotated with the originating channel's id.
#[derive(Debug, Copy, Clone)]
pub struct Message<const PL_SIZE: PayloadSize>
where
    [(); PL_SIZE as usize]:,
{
    pub port: ChannelId,
    pub payload: [u8; PL_SIZE as usize],
}

impl<const PL_SIZE: PayloadSize> Message<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    /// Gets the payload the message originated from.
    pub const fn into_inner(self) -> (ChannelId, [u8; PL_SIZE as usize]) {
        (self.port, self.payload)
    }
}

impl<const PL_SIZE: PayloadSize> Default for Message<PL_SIZE>
where
    [(); PL_SIZE as usize]:,
{
    fn default() -> Self {
        Message {
            port: ChannelId::from(0),
            payload: [0u8; PL_SIZE as usize],
        }
    }
}

/// Receives a message from a port.
pub trait ReceiveMessage {
    /// Receives a message from the hypervisor.
    fn receive_message<'a, const PL_SIZE: PayloadSize>(
        &self,
        message: &'a mut Message<PL_SIZE>,
    ) -> Result<&'a Message<PL_SIZE>, Error>
    where
        [(); PL_SIZE as usize]:;
}

impl<const MSG_SIZE: MessageSize, H: ApexSamplingPortP4> ReceiveMessage
    for (&ChannelId, &SamplingPortDestination<MSG_SIZE, H>)
{
    fn receive_message<'a, const PL_SIZE: PayloadSize>(
        &self,
        message: &'a mut Message<PL_SIZE>,
    ) -> Result<&'a Message<PL_SIZE>, Error>
    where
        [(); PL_SIZE as usize]:,
    {
        // TODO has to set link id.
        let (valid, _) = self.1.receive(&mut message.payload)?;
        if valid == Validity::Valid {
            message.port = *self.0;
            Ok(message)
        } else {
            Err(Error::InvalidData)
        }
    }
}

/// Looks up a sampling port source by its internal ID.
pub trait SamplingPortLookup<const MSG_SIZE: MessageSize, H: ApexSamplingPortP4> {
    /// Gets the sampling port source by the internal `id`.
    fn get_sampling_port_source<'a>(
        &'a self,
        id: &ChannelId,
    ) -> Option<&'a SamplingPortSource<MSG_SIZE, H>>
    where
        H: ApexSamplingPortP4;
}

impl<const MSG_SIZE: MessageSize, H: ApexSamplingPortP4, const PORTS: usize>
    SamplingPortLookup<MSG_SIZE, H>
    for LinearMap<ChannelId, SamplingPortSource<MSG_SIZE, H>, PORTS>
{
    fn get_sampling_port_source<'a>(
        &'a self,
        id: &ChannelId,
    ) -> Option<&'a SamplingPortSource<MSG_SIZE, H>>
    where
        H: ApexSamplingPortP4,
    {
        self.get(id)
    }
}

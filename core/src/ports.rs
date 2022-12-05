use crate::error::Error;
use crate::prelude::{Frame, PayloadSize, ReceiveFrame};
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

impl<const MSG_SIZE: MessageSize, H: ApexSamplingPortP4> ReceiveFrame
    for SamplingPortDestination<MSG_SIZE, H>
{
    fn receive_frame<'a, const PL_SIZE: PayloadSize>(
        &self,
        frame: &'a mut Frame<PL_SIZE>,
    ) -> Result<&'a Frame<PL_SIZE>, Error>
    where
        [(); PL_SIZE as usize]:,
    {
        let (valid, _) = self.receive(&mut frame.payload)?;
        if valid == Validity::Valid {
            Ok(frame)
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

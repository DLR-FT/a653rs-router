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

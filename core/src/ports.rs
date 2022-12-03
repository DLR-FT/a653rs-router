/// An ID of a hypervisor port.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

impl Default for ChannelId {
    fn default() -> ChannelId {
        ChannelId(0)
    }
}

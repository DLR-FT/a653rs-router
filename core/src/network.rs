use crate::ports::VirtualLinkId;

/// A virtual link.
#[derive(Debug, Clone)]
pub struct VirtualLink {
    /// See [VirtualLinkId].
    id: VirtualLinkId,
}

impl VirtualLink {
    /// Creates a new virtual link.
    pub fn new(id: VirtualLinkId) -> Self {
        VirtualLink { id }
    }
}

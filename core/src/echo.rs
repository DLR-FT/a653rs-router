//! Echo service
use apex_rs::prelude::*;
use apex_rs_postcard::error::SamplingRecvError;
use apex_rs_postcard::sampling::{SamplingPortDestinationExt, SamplingPortSourceExt};
use core::any::Any;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
/// Echo message
pub struct Echo {
    /// A sequence number.
    pub sequence: u32,

    /// The time at which the message has been created.
    pub when_ms: u64,
}

/// Samples a port and provides actions that the network partition can perform on it.
pub trait PortSampler<const T_SIZE: MessageSize, S, T>
where
    T: Any,
    S: ApexSamplingPortP4,
    [u8; T_SIZE as usize]:,
{
    /// Forwards the sample from this port to another sampling port.
    fn forward(
        &self,
        to: &SamplingPortSource<T_SIZE, S>,
    ) -> Result<(Validity, T), SamplingRecvError<T_SIZE>>;

    // TODO more methods
}

// <const PORT: SamplingPortId,
impl<const ECHO_SIZE: MessageSize, S> PortSampler<ECHO_SIZE, S, Echo>
    for SamplingPortDestination<ECHO_SIZE, S>
where
    S: ApexSamplingPortP4,
    [u8; ECHO_SIZE as usize]:,
{
    fn forward(
        &self,
        to: &SamplingPortSource<ECHO_SIZE, S>,
    ) -> Result<(Validity, Echo), SamplingRecvError<ECHO_SIZE>> {
        let result = self.recv_type::<Echo>();
        if let Ok((valid, data)) = result {
            if valid == Validity::Valid {
                _ = to.send_type(data);
            }
        }
        result
    }
}

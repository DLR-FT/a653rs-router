use a653rs::{bindings::*, prelude::*};
use core::{fmt::Display, marker::PhantomData, str::FromStr, time::Duration};

use crate::{
    config::{QueuingInCfg, QueuingOutCfg, SamplingInCfg, SamplingOutCfg},
    network::PayloadSize,
    prelude::*,
    router::{RouterInput, RouterOutput},
};

#[derive(Debug)]
pub(crate) enum Port<H: ApexQueuingPortP4 + ApexSamplingPortP4> {
    SamplingIn(SamplingIn<H>),
    SamplingOut(SamplingOut<H>),
    QueuingIn(QueuingIn<H>),
    QueuingOut(QueuingOut<H>),
}

#[derive(Debug)]
pub(crate) struct PortData {
    id: ApexLongInteger,
    msg_size: MessageSize,
}

/// Used for accessing a sampling port destination using the bindings
#[derive(Debug)]
pub(crate) struct SamplingIn<H: ApexSamplingPortP4> {
    _h: PhantomData<H>,
    inner: PortData,
}

/// Used for accessing a sampling port source using the bindings
#[derive(Debug)]
pub(crate) struct SamplingOut<H: ApexSamplingPortP4> {
    _h: PhantomData<H>,
    inner: PortData,
}

/// Used for accessing a queuing port source using the bindings
#[derive(Debug)]
pub(crate) struct QueuingOut<H: ApexQueuingPortP4> {
    _h: PhantomData<H>,
    inner: PortData,
}

/// Used for accessing a queuing port destination using the bindings
#[derive(Debug)]
pub(crate) struct QueuingIn<H: ApexQueuingPortP4> {
    _h: PhantomData<H>,
    inner: PortData,
}

impl<H: ApexSamplingPortP4> RouterInput for SamplingIn<H> {
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], PortError> {
        let buf = buf.validate_read(self.inner.msg_size)?;
        let (_val, len) = unsafe {
            <H as ApexSamplingPortP4>::read_sampling_message(self.inner.id, buf)
                .map_err(|_e| PortError::Receive)
        }?;
        Ok(&buf[..(len as usize)])
    }

    fn mtu(&self) -> PayloadSize {
        self.inner.msg_size as PayloadSize
    }
}

impl<H: ApexQueuingPortP4> RouterInput for QueuingIn<H> {
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], PortError> {
        let buf = buf.validate_read(self.inner.msg_size)?;
        let timeout = Duration::from_micros(10).as_nanos() as ApexSystemTime;
        let (val, _overflow) = unsafe {
            <H as ApexQueuingPortP4>::receive_queuing_message(self.inner.id, timeout, buf)
                .map_err(|_e| PortError::Receive)
        }?;
        Ok(&buf[..val as usize])
    }

    fn mtu(&self) -> PayloadSize {
        self.inner.msg_size as PayloadSize
    }
}

impl<H: ApexQueuingPortP4> RouterOutput for QueuingOut<H> {
    fn send(&self, _vl: &VirtualLinkId, buf: &[u8]) -> Result<(), PortError> {
        let buf = buf.validate_write(self.inner.msg_size)?;
        let timeout = Duration::from_micros(10).as_nanos() as ApexSystemTime;
        <H as ApexQueuingPortP4>::send_queuing_message(self.inner.id, buf, timeout)
            .map_err(|_e| PortError::Send)
    }

    fn mtu(&self) -> PayloadSize {
        self.inner.msg_size as PayloadSize
    }
}

impl<H: ApexSamplingPortP4> RouterOutput for SamplingOut<H> {
    fn send(&self, _vl: &VirtualLinkId, buf: &[u8]) -> Result<(), PortError> {
        let buf = buf.validate_write(self.inner.msg_size)?;
        <H as ApexSamplingPortP4>::write_sampling_message(self.inner.id, buf)
            .map_err(|_e| PortError::Send)
    }

    fn mtu(&self) -> PayloadSize {
        self.inner.msg_size as PayloadSize
    }
}

impl<H: ApexSamplingPortP4> SamplingIn<H> {
    pub(crate) fn create(name: PortName, value: SamplingInCfg) -> Result<Self, PortError> {
        let id = <H as ApexSamplingPortP4>::create_sampling_port(
            Name::from_str(&name)
                .map_err(|_e| PortError::Create)?
                .into(),
            value.msg_size,
            PortDirection::Destination,
            value.refresh_period.as_nanos() as i64,
        )
        .map_err(|_e| PortError::Create)?;
        Ok(Self {
            _h: Default::default(),
            inner: PortData {
                id,
                msg_size: value.msg_size,
            },
        })
    }
}

impl<H: ApexSamplingPortP4> SamplingOut<H> {
    pub(crate) fn create(name: PortName, value: SamplingOutCfg) -> Result<Self, PortError> {
        let id = <H as ApexSamplingPortP4>::create_sampling_port(
            Name::from_str(&name)
                .map_err(|_e| PortError::Create)?
                .into(),
            value.msg_size,
            PortDirection::Source,
            // Use some non-zero duration.
            // While refresh_period is ignored for the source
            // It may produce an error if set to zero
            SystemTime::Normal(Duration::from_nanos(1)).into(),
        )
        .map_err(|_e| PortError::Create)?;
        Ok(Self {
            _h: Default::default(),
            inner: PortData {
                id,
                msg_size: value.msg_size,
            },
        })
    }
}

impl<H: ApexQueuingPortP4> QueuingIn<H> {
    pub(crate) fn create(name: PortName, value: QueuingInCfg) -> Result<Self, PortError> {
        let id = <H as ApexQueuingPortP4>::create_queuing_port(
            Name::from_str(&name)
                .map_err(|_e| PortError::Create)?
                .into(),
            value.msg_size,
            value.msg_count,
            PortDirection::Destination,
            value.discipline.into(),
        )
        .map_err(|_e| PortError::Create)?;
        Ok(Self {
            _h: Default::default(),
            inner: PortData {
                id,
                msg_size: value.msg_size,
            },
        })
    }
}

impl<H: ApexQueuingPortP4> QueuingOut<H> {
    pub(crate) fn create(name: PortName, value: QueuingOutCfg) -> Result<Self, PortError> {
        let id = <H as ApexQueuingPortP4>::create_queuing_port(
            Name::from_str(&name)
                .map_err(|_e| PortError::Create)?
                .into(),
            value.msg_size,
            value.msg_count,
            PortDirection::Source,
            value.discipline.into(),
        )
        .map_err(|_e| PortError::Create)?;
        Ok(Self {
            _h: Default::default(),
            inner: PortData {
                id,
                msg_size: value.msg_size,
            },
        })
    }
}

impl<const M: MessageSize, S: ApexSamplingPortP4> RouterInput for SamplingPortDestination<M, S> {
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], PortError> {
        router_bench!(begin_apex_receive, self.id() as u16);
        let res = self.receive(buf);
        router_bench!(end_apex_receive, self.id() as u16);
        let (_val, data) = res.map_err(|_e| PortError::Receive)?;
        Ok(data)
    }

    fn mtu(&self) -> PayloadSize {
        M as PayloadSize
    }
}

impl<const M: MessageSize, S: ApexSamplingPortP4> RouterOutput for SamplingPortSource<M, S> {
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), PortError> {
        router_bench!(begin_apex_send, vl.0 as u16);
        let res = self.send(buf);
        router_bench!(end_apex_send, vl.0 as u16);
        res.map_err(|_e| PortError::Send)?;
        Ok(())
    }

    fn mtu(&self) -> PayloadSize {
        M as PayloadSize
    }
}

impl<const M: MessageSize, const R: MessageRange, Q: ApexQueuingPortP4> RouterInput
    for QueuingPortReceiver<M, R, Q>
{
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], PortError> {
        let timeout = SystemTime::Normal(Duration::from_micros(10));
        router_bench!(begin_apex_send, self.id() as u16);
        let res = self.receive(buf, timeout);
        router_bench!(end_apex_send, self.id() as u16);
        let (buf, _overflow) = res.map_err(|_e| PortError::Receive)?;
        Ok(buf)
    }

    fn mtu(&self) -> PayloadSize {
        M as PayloadSize
    }
}

impl<const M: MessageSize, const R: MessageRange, Q: ApexQueuingPortP4> RouterOutput
    for QueuingPortSender<M, R, Q>
{
    fn send(&self, vl: &VirtualLinkId, buf: &[u8]) -> Result<(), PortError> {
        let timeout = SystemTime::Normal(Duration::from_micros(10));
        router_bench!(begin_apex_send, vl.0 as u16);
        let res = self.send(buf, timeout);
        router_bench!(end_apex_send, vl.0 as u16);
        res.map_err(|_e| PortError::Send)?;
        Ok(())
    }

    fn mtu(&self) -> PayloadSize {
        M as PayloadSize
    }
}

trait BufferExt {
    fn validate_read(&mut self, size: MessageSize) -> Result<&mut Self, PortError>;

    /// Validate a buffer to be at most as long as the given usize.  
    /// If not returns [Self] with the length of the passed buffer
    fn validate_write(&self, size: MessageSize) -> Result<&Self, PortError>;
}

impl BufferExt for [ApexByte] {
    fn validate_read(&mut self, size: MessageSize) -> Result<&mut Self, PortError> {
        if usize::try_from(size)
            .map(|ss| self.len() < ss)
            .unwrap_or(true)
        {
            return Err(PortError::Receive);
        }
        Ok(self)
    }

    fn validate_write(&self, size: MessageSize) -> Result<&Self, PortError> {
        if usize::try_from(size)
            .map(|ss| self.len() > ss)
            .unwrap_or(false)
            || self.is_empty()
        {
            return Err(PortError::Send);
        }
        Ok(self)
    }
}

/// An error occured while reading or writing a port of the router.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortError {
    /// Failed to send from router output
    Send,

    /// Failed to receive from router input
    Receive,

    /// Creating of the router input / output failed
    Create,
}

impl Display for PortError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self {
            Self::Send => write!(f, "Failed to send from router output"),
            Self::Receive => write!(f, "Failed to receive from router input"),
            Self::Create => write!(f, "Failed to create router port"),
        }
    }
}

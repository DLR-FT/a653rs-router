#![no_std]

pub enum TraceEvent {
    Noop,
    NetworkSend(u16),
    NetworkReceive(u16),
    ApexSend(u16),
    ApexReceive(u16),
    ForwardFromNetwork(u16),
    ForwardFromApex(u16),
    ForwardToNetwork(u16),
    ForwardToApex(u16),
    VirtualLinkScheduled(u16),
    Echo(EchoEvent),
}

impl TraceEvent {
    /// `Self::Echo(EchoEvent::EchoReplySend)`
    pub fn echo_req_send() -> Self {
        Self::Echo(EchoEvent::EchoReplySend)
    }

    /// `Self::Echo(EchoEvent::EchoRequestReceived`
    pub fn echo_req_rcvd() -> Self {
        Self::Echo(EchoEvent::EchoRequestReceived)
    }

    /// `Self::Echo(EchoEvent::EchoReplySend`
    pub fn echo_repl_send() -> Self {
        Self::Echo(EchoEvent::EchoReplySend)
    }

    /// `Self::Echo(EchoEvent::EchoReplyReceived)`
    pub fn echo_repl_rcvd() -> Self {
        Self::Echo(EchoEvent::EchoReplyReceived)
    }
}

pub enum EchoEvent {
    EchoRequestSend,
    EchoRequestReceived,
    EchoReplySend,
    EchoReplyReceived,
}

impl From<EchoEvent> for u16 {
    fn from(value: EchoEvent) -> Self {
        match value {
            EchoEvent::EchoRequestSend => 0,
            EchoEvent::EchoRequestReceived => 1,
            EchoEvent::EchoReplySend => 2,
            EchoEvent::EchoReplyReceived => 3,
        }
    }
}

impl From<TraceEvent> for u16 {
    fn from(value: TraceEvent) -> Self {
        // First 4 bits for event, second 4 bits for data.
        // There are 16 GPIOs, but only so many probes.
        let (event, data) = match value {
            TraceEvent::Noop => (0, 0x0),
            TraceEvent::NetworkSend(network_interface) => (1, network_interface),
            TraceEvent::NetworkReceive(network_interface) => (2, network_interface),
            TraceEvent::ApexSend(virtual_link) => (3, virtual_link),
            TraceEvent::ApexReceive(virtual_link) => (4, virtual_link),
            TraceEvent::ForwardFromNetwork(virtual_link) => (5, virtual_link),
            TraceEvent::ForwardFromApex(virtual_link) => (6, virtual_link),
            TraceEvent::ForwardToNetwork(virtual_link) => (7, virtual_link),
            TraceEvent::ForwardToApex(virtual_link) => (8, virtual_link),
            TraceEvent::VirtualLinkScheduled(virtual_link) => (9, virtual_link),
            TraceEvent::Echo(echo) => (10, u16::from(echo)),
        };
        //debug_assert_eq!(
        //    0,
        //    ((0b11_1111 << 10) | 0b11) & data,
        //    "Data was wider than 6 bit"
        //);
        (event << 4) | data
    }
}

#[macro_export]
macro_rules! gpio_trace {
    ( $arg:expr ) => {{
        $crate::__private_api_trace($arg)
    }};
}

#[doc(hidden)]
pub fn __private_api_trace(arg: TraceEvent) {
    tracer().trace(u16::from(arg))
}

pub trait Tracer: Send + Sync {
    fn trace(&self, val: u16);
}

pub static mut TRACER: &dyn Tracer = &NoopTracer;

pub struct NoopTracer;

impl Tracer for NoopTracer {
    fn trace(&self, _val: u16) {}
}

pub fn tracer() -> &'static dyn Tracer {
    unsafe { TRACER }
}

pub fn set_tracer(tracer: &'static dyn Tracer) {
    unsafe { TRACER = tracer }
}

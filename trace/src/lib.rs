#![no_std]

pub enum TraceEvent {
    Begin(TraceType),
    End(TraceType),
}

impl TraceEvent {
    // Beginnings

    pub const fn begin_network_send(network_interface: u16) -> Self {
        Self::Begin(TraceType::NetworkSend(network_interface))
    }

    pub const fn begin_network_receive(network_interface: u16) -> Self {
        Self::Begin(TraceType::NetworkReceive(network_interface))
    }

    pub const fn begin_apex_send(network_interface: u16) -> Self {
        Self::Begin(TraceType::NetworkReceive(network_interface))
    }

    pub const fn begin_apex_receive(virtual_link: u16) -> Self {
        Self::Begin(TraceType::NetworkReceive(virtual_link))
    }

    pub const fn begin_forward_from_network(virtual_link: u16) -> Self {
        Self::Begin(TraceType::ForwardFromNetwork(virtual_link))
    }

    pub const fn begin_forward_from_apex(virtual_link: u16) -> Self {
        Self::Begin(TraceType::ForwardFromApex(virtual_link))
    }

    pub const fn begin_forward_to_network(virtual_link: u16) -> Self {
        Self::Begin(TraceType::ForwardToNetwork(virtual_link))
    }

    pub const fn begin_forward_to_apex(virtual_link: u16) -> Self {
        Self::Begin(TraceType::ForwardToApex(virtual_link))
    }

    pub const fn begin_virtual_link_scheduled(virtual_link: u16) -> Self {
        Self::Begin(TraceType::VirtualLinkScheduled(virtual_link))
    }

    pub const fn begin_echo_request_send() -> Self {
        Self::Begin(TraceType::Echo(EchoEvent::RequestSend))
    }

    pub const fn begin_echo_reply_send() -> Self {
        Self::Begin(TraceType::Echo(EchoEvent::ReplySend))
    }

    pub const fn begin_echo_request_received() -> Self {
        Self::Begin(TraceType::Echo(EchoEvent::RequestReceived))
    }

    pub const fn begin_echo_reply_received() -> Self {
        Self::Begin(TraceType::Echo(EchoEvent::ReplyReceived))
    }

    // Ends

    pub const fn end_network_send(network_interface: u16) -> Self {
        Self::End(TraceType::NetworkSend(network_interface))
    }

    pub const fn end_network_receive(network_interface: u16) -> Self {
        Self::End(TraceType::NetworkReceive(network_interface))
    }

    pub const fn end_apex_send(network_interface: u16) -> Self {
        Self::End(TraceType::NetworkReceive(network_interface))
    }

    pub const fn end_apex_receive(virtual_link: u16) -> Self {
        Self::End(TraceType::NetworkReceive(virtual_link))
    }

    pub const fn end_forward_from_network(virtual_link: u16) -> Self {
        Self::End(TraceType::ForwardFromNetwork(virtual_link))
    }

    pub const fn end_forward_from_apex(virtual_link: u16) -> Self {
        Self::End(TraceType::ForwardFromApex(virtual_link))
    }

    pub const fn end_forward_to_network(virtual_link: u16) -> Self {
        Self::End(TraceType::ForwardToNetwork(virtual_link))
    }

    pub const fn end_forward_to_apex(virtual_link: u16) -> Self {
        Self::End(TraceType::ForwardToApex(virtual_link))
    }

    pub const fn end_virtual_link_scheduled(virtual_link: u16) -> Self {
        Self::End(TraceType::VirtualLinkScheduled(virtual_link))
    }

    pub const fn end_echo_request_send() -> Self {
        Self::End(TraceType::Echo(EchoEvent::RequestSend))
    }

    pub const fn end_echo_reply_send() -> Self {
        Self::End(TraceType::Echo(EchoEvent::ReplySend))
    }

    pub const fn end_echo_request_received() -> Self {
        Self::End(TraceType::Echo(EchoEvent::RequestReceived))
    }

    pub const fn end_echo_reply_received() -> Self {
        Self::End(TraceType::Echo(EchoEvent::ReplyReceived))
    }
}

pub enum TraceType {
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

pub enum EchoEvent {
    RequestSend,
    RequestReceived,
    ReplySend,
    ReplyReceived,
}

impl From<EchoEvent> for u16 {
    fn from(value: EchoEvent) -> Self {
        match value {
            EchoEvent::RequestSend => 0,
            EchoEvent::RequestReceived => 1,
            EchoEvent::ReplySend => 2,
            EchoEvent::ReplyReceived => 3,
        }
    }
}

impl From<TraceEvent> for u16 {
    fn from(value: TraceEvent) -> Self {
        // First bit for begin / end, next 4 bits for event, last 3 bits for data.
        // There are 16 GPIOs, but only so many probes.
        let (begin_end, e_type) = match value {
            TraceEvent::Begin(e_type) => (0, e_type),
            TraceEvent::End(e_type) => (1, e_type),
        };
        let (event, data) = match e_type {
            TraceType::Noop => (0, 0x0),
            TraceType::NetworkSend(network_interface) => (1, network_interface),
            TraceType::NetworkReceive(network_interface) => (2, network_interface),
            TraceType::ApexSend(virtual_link) => (3, virtual_link),
            TraceType::ApexReceive(virtual_link) => (4, virtual_link),
            TraceType::ForwardFromNetwork(virtual_link) => (5, virtual_link),
            TraceType::ForwardFromApex(virtual_link) => (6, virtual_link),
            TraceType::ForwardToNetwork(virtual_link) => (7, virtual_link),
            TraceType::ForwardToApex(virtual_link) => (8, virtual_link),
            TraceType::VirtualLinkScheduled(virtual_link) => (9, virtual_link),
            TraceType::Echo(echo) => (10, u16::from(echo)),
        };
        begin_end << 7 | (event << 3) | data >> 1
    }
}

#[macro_export]
macro_rules! gpio_trace {
    ( $arg0:ident ) => {{
        $crate::__private_api_trace($crate::TraceEvent::$arg0())
    }};
    ( $arg0:ident, $arg1:expr ) => {{
        $crate::__private_api_trace($crate::TraceEvent::$arg0($arg1))
    }};
}

#[doc(hidden)]
pub fn __private_api_trace(arg0: TraceEvent) {
    tracer().trace(u16::from(arg0))
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

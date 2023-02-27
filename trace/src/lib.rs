#![no_std]

use core::debug_assert_eq;

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
}

impl From<TraceEvent> for u16 {
    fn from(value: TraceEvent) -> Self {
        // First 6 bit are for event type. Remaining 10 bit are for data
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
        };
        debug_assert_eq!(
            0,
            (0b111111 << 10) & (data as u16),
            "Data was wider than 10 bit"
        );
        (event << 10) | data as u16
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

#![no_std]

#[macro_export]
macro_rules! gpio_trace {
    ( $arg:expr ) => {{
        $crate::__private_api_trace($arg)
    }};
}

#[doc(hidden)]
pub fn __private_api_trace(arg: u8) {
    tracer().trace(arg)
}

pub trait Tracer: Send + Sync {
    fn trace(&self, val: u8);
}

pub static mut TRACER: &dyn Tracer = &NoopTracer;

pub struct NoopTracer;

impl Tracer for NoopTracer {
    fn trace(&self, _val: u8) {}
}

pub fn tracer() -> &'static dyn Tracer {
    unsafe { TRACER }
}

pub fn set_tracer(tracer: &'static dyn Tracer) {
    unsafe { TRACER = tracer }
}

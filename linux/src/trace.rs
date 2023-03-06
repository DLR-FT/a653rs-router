use log::trace;
use small_trace::{TraceEvent, Tracer};

pub struct LinuxTracer;

impl Tracer for LinuxTracer {
    fn trace(&self, val: TraceEvent) {
        trace!("small_trace: {val:?}")
    }
}

use small_trace::{TraceEvent, Tracer};

pub struct LinuxTracer;

impl Tracer for LinuxTracer {
    fn trace(&self, val: TraceEvent) {
        router_trace!("small_trace: {val:?}")
    }
}

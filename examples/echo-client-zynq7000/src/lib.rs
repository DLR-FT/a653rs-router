#![no_std]

use a653rs::prelude::*;
use a653rs_xng::apex::XngHypervisor;
use apex_echo::queuing::*;
use log::info;
use once_cell::unsync::OnceCell;
use small_trace_gpio::GpioTracer;
use xng_rs_log::XalLogger;

const ECHO_SIZE: MessageSize = 100;
const FIFO_DEPTH: MessageRange = 10;

static mut SENDER: OnceCell<QueuingPortSender<ECHO_SIZE, FIFO_DEPTH, XngHypervisor>> =
    OnceCell::new();

static mut RECEIVER: OnceCell<QueuingPortReceiver<ECHO_SIZE, FIFO_DEPTH, XngHypervisor>> =
    OnceCell::new();

static TRACER: GpioTracer = GpioTracer::new();

#[no_mangle]
pub extern "C" fn main() {
    TRACER.init();
    small_trace::set_tracer(&TRACER);
    // The logger should be disabled during measurements
    unsafe { log::set_logger_racy(&XalLogger).unwrap() };
    log::set_max_level(log::LevelFilter::Info);
    info!("Echo client main");
    let partition = QueuingPeriodicEchoPartition::new(
        unsafe { &SENDER },
        unsafe { &RECEIVER },
        entry_point_periodic,
        entry_point_aperiodic,
    );
    partition.run()
}

extern "C" fn entry_point_periodic() {
    QueuingEchoSender::run(unsafe { SENDER.get_mut().unwrap() });
}

extern "C" fn entry_point_aperiodic() {
    QueuingEchoReceiver::run(unsafe { RECEIVER.get_mut().unwrap() });
}

#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

mod client;
mod queuing;
mod server;
mod server_queuing;

#[cfg(any(
    all(feature = "dummy", any(feature = "linux", feature = "xng")),
    all(feature = "linux", any(feature = "xng", feature = "dummy")),
    all(feature = "xng", any(feature = "dummy", feature = "linux")),
))]
compile_error!("The features dummy, linux and xng are mutually exclusive, because they are meant for different platforms.");

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
use a653rs::prelude::*;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
use log::info;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
use once_cell::unsync::OnceCell;

#[cfg(feature = "dummy")]
type Hypervisor = dummy_hypervisor::DummyHypervisor;

#[cfg(feature = "linux")]
type Hypervisor = a653rs_linux::partition::ApexLinuxPartition;

#[cfg(feature = "xng")]
type Hypervisor = a653rs_xng::apex::XngHypervisor;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
const ECHO_SIZE: MessageSize = 100;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
const FIFO_DEPTH: MessageRange = 10;

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
static mut SENDER: OnceCell<QueuingPortSender<ECHO_SIZE, FIFO_DEPTH, Hypervisor>> = OnceCell::new();

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
static mut RECEIVER: OnceCell<QueuingPortReceiver<ECHO_SIZE, FIFO_DEPTH, Hypervisor>> =
    OnceCell::new();

#[cfg(feature = "xng")]
static LOGGER: xng_rs_log::XalLogger = xng_rs_log::XalLogger;

#[cfg(feature = "xng")]
static TRACER: small_trace_gpio::GpioTracer = small_trace_gpio::GpioTracer::new();

#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
pub fn run() {
    #[cfg(feature = "xng")]
    {
        TRACER.init();
        small_trace::set_tracer(&TRACER);
    }
    #[cfg(feature = "xng")]
    {
        unsafe { log::set_logger_racy(&XalLogger).unwrap() };
    }
    #[cfg(feature = "linux")]
    {
        a653rs_linux::partition::ApexLogger::install_panic_hook();
        a653rs_linux::partition::ApexLogger::install_logger(log::LevelFilter::Info).unwrap();
    }
    log::set_max_level(log::LevelFilter::Info);
    #[cfg(feature = "client")]
    {
        info!("Echo client main");
        let partition = queuing::QueuingPeriodicEchoPartition::new(
            unsafe { &SENDER },
            unsafe { &RECEIVER },
            entry_point_periodic,
            entry_point_aperiodic,
        );
        partition.run()
    }

    #[cfg(feature = "server")]
    {
        use server::EchoServerPartition;

        info!("Echo server main");
        let partition = EchoServerPartition::new(
            unsafe { &SENDER },
            unsafe { &RECEIVER },
            entry_point_periodic,
            entry_point_aperiodic,
        );
        partition.run()
    }
}

#[cfg(all(
    feature = "client",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
extern "C" fn entry_point_periodic() {
    QueuingEchoSender::run(unsafe { SENDER.get_mut().unwrap() });
}

#[cfg(all(
    feature = "client",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
extern "C" fn entry_point_aperiodic() {
    QueuingEchoReceiver::run(unsafe { RECEIVER.get_mut().unwrap() });
}

#[cfg(all(
    feature = "server",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
extern "C" fn entry_point_periodic() {
    QueuingEchoSender::run(unsafe { SENDER.get_mut().unwrap() });
}

#[cfg(all(
    feature = "server",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
extern "C" fn entry_point_aperiodic() {
    EchoServerProcess::run(unsafe { SENDER.get_mut().unwrap() }, unsafe {
        RECEIVER.get_mut().unwrap()
    });
}

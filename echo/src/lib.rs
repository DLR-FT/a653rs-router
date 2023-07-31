#![no_std]
#![allow(incomplete_features)]
#![allow(unused_imports)]
#![feature(generic_const_exprs)]

#[cfg(all(feature = "sampling", feature = "client"))]
mod client;

#[cfg(all(feature = "queuing", feature = "client"))]
mod queuing;

#[cfg(all(feature = "sampling", feature = "server"))]
mod server;

#[cfg(all(feature = "queuing", feature = "server"))]
mod server_queuing;

#[cfg(any(
    all(feature = "dummy", any(feature = "linux", feature = "xng")),
    all(feature = "linux", any(feature = "xng", feature = "dummy")),
    all(feature = "xng", any(feature = "dummy", feature = "linux")),
))]
compile_error!("The features dummy, linux and xng are mutually exclusive, because they are meant for different platforms.");

#[cfg(all(feature = "sampling", feature = "queuing"))]
compile_error!("The features `sampling` and `queuing` are mutually exclusive.");

use a653rs::prelude::*;
use log::{info, trace};
use once_cell::unsync::OnceCell;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;

#[allow(dead_code)]
#[cfg(feature = "dummy")]
type Hypervisor = dummy_hypervisor::DummyHypervisor;

#[allow(dead_code)]
#[cfg(feature = "linux")]
type Hypervisor = a653rs_linux::partition::ApexLinuxPartition;

#[allow(dead_code)]
#[cfg(feature = "xng")]
type Hypervisor = a653rs_xng::apex::XngHypervisor;

#[allow(dead_code)]
#[cfg(any(feature = "dummy", feature = "xng", feature = "linux"))]
const ECHO_SIZE: MessageSize = 1000;

#[cfg(all(
    feature = "queuing",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
const FIFO_DEPTH: MessageRange = 10;

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
/// Echo message
pub struct Echo {
    /// A sequence number.
    pub sequence: u32,

    /// The time at which the message has been created.
    pub when_us: u64,
}

#[cfg(all(
    feature = "queuing",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
static mut SENDER: OnceCell<QueuingPortSender<ECHO_SIZE, FIFO_DEPTH, Hypervisor>> = OnceCell::new();

#[cfg(all(
    feature = "queuing",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
static mut RECEIVER: OnceCell<QueuingPortReceiver<ECHO_SIZE, FIFO_DEPTH, Hypervisor>> =
    OnceCell::new();

#[cfg(all(
    feature = "sampling",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
static mut SENDER: OnceCell<SamplingPortSource<ECHO_SIZE, Hypervisor>> = OnceCell::new();

#[cfg(all(
    feature = "sampling",
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
static mut RECEIVER: OnceCell<SamplingPortDestination<ECHO_SIZE, Hypervisor>> = OnceCell::new();

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
        unsafe { log::set_logger_racy(&LOGGER).unwrap() };
        log::set_max_level(LOG_LEVEL);
    }
    #[cfg(feature = "linux")]
    {
        a653rs_linux::partition::ApexLogger::install_panic_hook();
        a653rs_linux::partition::ApexLogger::install_logger(LOG_LEVEL).unwrap();
    }
    #[cfg(feature = "client")]
    {
        info!("Echo client main");
        #[cfg(feature = "sampling")]
        {
            let partition = crate::client::PeriodicEchoPartition::new(
                unsafe { &SENDER },
                unsafe { &RECEIVER },
                entry_point_periodic,
                entry_point_aperiodic,
            );
            trace!("Echo client main: running partition");
            partition.run()
        }
        #[cfg(feature = "queuing")]
        {
            let partition = queuing::QueuingPeriodicEchoPartition::new(
                unsafe { &SENDER },
                unsafe { &RECEIVER },
                entry_point_periodic,
                entry_point_aperiodic,
            );
            trace!("Echo server main: running partition");
            partition.run()
        }
    }

    #[cfg(feature = "server")]
    {
        use server::EchoServerPartition;

        info!("Echo server main");
        let partition = EchoServerPartition::new(
            unsafe { &SENDER },
            unsafe { &RECEIVER },
            entry_point_aperiodic,
        );
        partition.run()
    }
}

#[cfg(all(
    feature = "client",
    any(feature = "sampling", feature = "queuing"),
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
extern "C" fn entry_point_periodic() {
    #[cfg(feature = "queuing")]
    use crate::queuing::QueuingEchoSender as Sender;

    #[cfg(feature = "sampling")]
    use crate::client::EchoSenderProcess as Sender;

    Sender::run(unsafe { SENDER.get_mut().unwrap() });
}

#[cfg(all(
    feature = "client",
    any(feature = "sampling", feature = "queuing"),
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
extern "C" fn entry_point_aperiodic() {
    #[cfg(feature = "queuing")]
    use crate::queuing::QueuingEchoReceiver as Receiver;

    #[cfg(feature = "sampling")]
    use crate::client::EchoReceiverProcess as Receiver;

    #[cfg(any(feature = "sampling", feature = "queuing"))]
    Receiver::run(unsafe { RECEIVER.get_mut().unwrap() });
}

#[cfg(all(
    feature = "server",
    any(feature = "sampling", feature = "queuing"),
    any(feature = "dummy", feature = "xng", feature = "linux")
))]
extern "C" fn entry_point_aperiodic() {
    #[cfg(feature = "queuing")]
    use crate::server_queuing::EchoServerProcess as Receiver;

    #[cfg(feature = "sampling")]
    use crate::server::EchoServerProcess as Receiver;

    #[cfg(any(feature = "sampling", feature = "queuing"))]
    Receiver::run(unsafe { SENDER.get_mut().unwrap() }, unsafe {
        RECEIVER.get_mut().unwrap()
    });
}

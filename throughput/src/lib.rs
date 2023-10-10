#![no_std]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use a653rs::bindings::{ApexPartitionP4, ApexProcessP4, ApexQueuingPortP4};
use a653rs::prelude::*;

#[cfg(any(feature = "sender", feature = "receiver"))]
use a653rs_xng::apex::XngHypervisor;

use core::str::FromStr;
use core::time::Duration;
use log::info;
use once_cell::unsync::OnceCell;

#[cfg(any(feature = "sender", feature = "receiver"))]
use xng_rs_log::XalLogger;

// Maximum from XNG header files
#[cfg(any(feature = "sender", feature = "receiver"))]
const MSG: MessageSize = 8192;
#[cfg(any(feature = "sender", feature = "receiver"))]
const FIFO: MessageRange = 256;

// As defined in LithOS constraints
#[cfg(any(feature = "sender", feature = "receiver"))]
const INTERVAL: Duration = Duration::from_millis(60);
#[cfg(any(feature = "sender", feature = "receiver"))]
static LOGGER: XalLogger = XalLogger;

#[cfg(any(feature = "sender", feature = "receiver"))]
type Hypervisor = XngHypervisor;

#[derive(Debug)]
pub struct TrafficSender<
    const M: MessageSize,
    const F: MessageRange,
    H: ApexQueuingPortP4 + 'static,
> {
    sender: &'static OnceCell<QueuingPortSender<M, F, H>>,
    entry_point: extern "C" fn(),
    interval: SystemTime,
}

pub fn send<const M: MessageSize, const F: MessageRange, H: ApexQueuingPortP4 + ApexTimeP4Ext>(
    port: &QueuingPortSender<M, F, H>,
    interval: &SystemTime,
) -> !
where
    [u8; M as usize]:,
{
    loop {
        // Fill queue until limit defined by XNG
        while port.send(&[b'A'; M as usize], interval.clone()).is_ok() {}
        <H as ApexTimeP4Ext>::periodic_wait().unwrap();
    }
}

impl<const M: MessageSize, const F: MessageRange, H: ApexQueuingPortP4 + 'static>
    TrafficSender<M, F, H>
{
    pub fn new(
        sender: &'static OnceCell<QueuingPortSender<M, F, H>>,
        entry_point: extern "C" fn(),
        interval: SystemTime,
    ) -> Self {
        Self {
            sender,
            entry_point,
            interval,
        }
    }
}

impl<
        const M: MessageSize,
        const F: MessageRange,
        H: ApexQueuingPortP4 + ApexPartitionP4 + ApexProcessP4 + 'static,
    > Partition<H> for TrafficSender<M, F, H>
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        info!("Starting throughput sender");
        let send_port = ctx
            .create_queuing_port_sender(
                Name::from_str("TrafficS").unwrap(),
                QueuingDiscipline::Fifo,
            )
            .unwrap();
        _ = self.sender.set(send_port);
        ctx.create_process(ProcessAttribute {
            period: self.interval.clone(),
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point,
            stack_size: 10000,
            base_priority: 5,
            deadline: Deadline::Soft,
            name: Name::from_str("TrafficS").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap();
    }

    fn warm_start(&self, ctx: &mut StartContext<H>) {
        self.cold_start(ctx);
    }
}

#[derive(Debug)]
pub struct TrafficReceiverPartition<
    const M: MessageSize,
    const F: MessageRange,
    H: ApexQueuingPortP4 + 'static,
> {
    port: &'static OnceCell<QueuingPortReceiver<M, F, H>>,
    receive_entry_point: extern "C" fn(),
    log_entry_point: extern "C" fn(),
}

#[derive(Debug, Default)]
pub struct TrafficReceiver<const M: MessageSize, const F: MessageRange> {
    count_received_data: u64,
}

impl<const M: MessageSize, const F: MessageRange> TrafficReceiver<M, F>
where
    [u8; M as usize]:,
{
    pub const fn new() -> Self {
        Self {
            count_received_data: 0u64,
        }
    }

    pub fn receive<H: ApexQueuingPortP4 + ApexTimeP4Ext>(
        &mut self,
        port: &QueuingPortReceiver<M, F, H>,
        interval: &SystemTime,
    ) -> ! {
        let buf = &mut [0u8; M as usize];
        loop {
            match port.receive(buf, interval.clone()) {
                Ok(msg) => {
                    self.count_received_data += msg.len() as u64;
                }
                Err(_e) => {
                    // Read as fast as possible
                }
            }
        }
    }

    pub fn log<H: ApexQueuingPortP4 + ApexTimeP4Ext>(&self) -> ! {
        loop {
            info!("Received: {}", self.count_received_data);
            <H as ApexTimeP4Ext>::periodic_wait().unwrap();
        }
    }
}

impl<const M: MessageSize, const F: MessageRange, H: ApexQueuingPortP4 + 'static>
    TrafficReceiverPartition<M, F, H>
{
    pub fn new(
        port: &'static OnceCell<QueuingPortReceiver<M, F, H>>,
        receive_entry_point: extern "C" fn(),
        log_entry_point: extern "C" fn(),
    ) -> Self {
        Self {
            port,
            receive_entry_point,
            log_entry_point,
        }
    }
}

impl<
        const M: MessageSize,
        const F: MessageRange,
        H: ApexQueuingPortP4 + ApexPartitionP4 + ApexProcessP4 + 'static,
    > Partition<H> for TrafficReceiverPartition<M, F, H>
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        info!("Starting throughput receiver");
        let port = ctx
            .create_queuing_port_receiver(
                Name::from_str("TrafficR").unwrap(),
                QueuingDiscipline::Fifo,
            )
            .unwrap();
        _ = self.port.set(port);
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Infinite,
            time_capacity: SystemTime::Infinite,
            entry_point: self.receive_entry_point, // receive
            stack_size: 10000,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("TrafficR").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap();
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::from_secs(1)),
            time_capacity: SystemTime::Infinite,
            entry_point: self.log_entry_point, // log
            stack_size: 10000,
            base_priority: 5,
            deadline: Deadline::Soft,
            name: Name::from_str("TrafficL").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap();
    }

    fn warm_start(&self, ctx: &mut StartContext<H>) {
        self.cold_start(ctx);
    }
}

#[cfg(any(feature = "sender", feature = "receiver"))]
fn setup_logger() {
    unsafe { log::set_logger_racy(&LOGGER).unwrap() };
    log::set_max_level(log::LevelFilter::Trace);
}

#[cfg(feature = "sender")]
static mut SENDER: OnceCell<QueuingPortSender<MSG, FIFO, Hypervisor>> = OnceCell::new();

#[cfg(feature = "sender")]
#[no_mangle]
pub extern "C" fn main() {
    setup_logger();
    let part = unsafe { TrafficSender::new(&SENDER, sender, SystemTime::Normal(INTERVAL)) };
    part.run();
}

#[cfg(feature = "sender")]
#[no_mangle]
pub extern "C" fn sender() {
    send::<MSG, FIFO, Hypervisor>(
        unsafe { SENDER.get_mut().unwrap() },
        &SystemTime::Normal(INTERVAL),
    );
}

#[cfg(feature = "receiver")]
static mut RECEIVER_PORT: OnceCell<QueuingPortReceiver<MSG, FIFO, Hypervisor>> = OnceCell::new();

#[cfg(feature = "receiver")]
#[no_mangle]
pub extern "C" fn main() {
    setup_logger();
    let part = unsafe { TrafficReceiverPartition::new(&RECEIVER_PORT, receiver, logger) };
    part.run();
}

#[cfg(feature = "receiver")]
static mut RECEIVER: TrafficReceiver<MSG, FIFO> = TrafficReceiver::new();

#[cfg(feature = "receiver")]
#[no_mangle]
pub extern "C" fn receiver() {
    unsafe {
        RECEIVER.receive::<Hypervisor>(
            RECEIVER_PORT.get_mut().unwrap(),
            &SystemTime::Normal(INTERVAL),
        )
    };
}

#[cfg(feature = "receiver")]
#[no_mangle]
pub extern "C" fn logger() {
    unsafe { RECEIVER.log::<Hypervisor>() }
}

#![no_std]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use apex_rs::prelude::*;
use core::{str::FromStr, time::Duration};
use log::{debug, error, info, warn};
use once_cell::unsync::OnceCell;

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
    port: &mut QueuingPortSender<M, F, H>,
    interval: &SystemTime,
) -> !
where
    [u8; M as usize]:,
{
    loop {
        // Fill queue until limit defined by XNG
        for _ in 0..F {
            if let Err(e) = port.send(&[b'A'; M as usize], interval.clone()) {
                warn!("Failed to send traffic: {e:?}");
            }
        }
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
        let send_port = ctx
            .create_queuing_port_sender(
                Name::from_str("TrafficS").unwrap(),
                QueuingDiscipline::FIFO,
                F,
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
        port: &mut QueuingPortReceiver<M, F, H>,
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
        let port = ctx
            .create_queuing_port_receiver(
                Name::from_str("TrafficR").unwrap(),
                QueuingDiscipline::FIFO,
                F,
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

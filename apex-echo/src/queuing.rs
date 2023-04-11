use crate::client::Echo;

use apex_rs::bindings::*;
use apex_rs::prelude::*;
use apex_rs_postcard::prelude::*;
use core::str::FromStr;
use core::time::Duration;
use log::{debug, error, info, trace, warn};
use once_cell::unsync::OnceCell;
use small_trace::small_trace;

#[derive(Debug)]
pub struct QueuingEchoSender<const ECHO_SIZE: MessageSize, const FIFO_DEPTH: MessageRange>;

impl<const ECHO_SIZE: MessageSize, const FIFO_DEPTH: MessageRange>
    QueuingEchoSender<ECHO_SIZE, FIFO_DEPTH>
where
    [u8; ECHO_SIZE as usize]:,
{
    pub fn run<H: ApexQueuingPortP4 + ApexTimeP4Ext>(
        port: &mut QueuingPortSender<ECHO_SIZE, FIFO_DEPTH, H>,
    ) -> ! {
        info!("Running echo client periodic process");
        let mut i: u32 = 0;
        loop {
            i += 1;
            let now = <H as ApexTimeP4Ext>::get_time().unwrap_duration();
            let data = Echo {
                sequence: i,
                when_us: now.as_micros() as u64,
            };
            small_trace!(begin_echo_request_send);
            let result = port.send_type(data, SystemTime::Normal(Duration::from_micros(10)));
            small_trace!(end_echo_request_send);
            match result {
                Ok(_) => {
                    debug!(
                        "EchoRequest: seqnr = {:?}, time = {:?} us",
                        data.sequence, data.when_us
                    );
                }
                Err(SendError::Apex(Error::TimedOut)) => {
                    warn!("Timed out while trying to send echo request");
                }
                Err(_) => {
                    error!("Failed to send EchoRequest");
                }
            }
            <H as ApexTimeP4Ext>::periodic_wait().unwrap();
        }
    }
}

#[derive(Debug)]
pub struct QueuingEchoReceiver<const ECHO_SIZE: MessageSize, const FIFO_DEPTH: MessageRange>;

impl<const ECHO_SIZE: MessageSize, const FIFO_DEPTH: MessageRange>
    QueuingEchoReceiver<ECHO_SIZE, FIFO_DEPTH>
where
    [u8; ECHO_SIZE as usize]:,
{
    pub fn run<H: ApexQueuingPortP4 + ApexTimeP4Ext>(
        port: &mut QueuingPortReceiver<ECHO_SIZE, FIFO_DEPTH, H>,
    ) -> ! {
        let mut last = 0;
        loop {
            trace!("Running echo client aperiodic process");
            let result = port.recv_type::<Echo>(SystemTime::Normal(Duration::from_millis(10)));
            let now = <H as ApexTimeP4Ext>::get_time().unwrap_duration();

            match result {
                Ok(data) => {
                    small_trace!(begin_echo_reply_received);
                    trace!("Received reply: {data:?}");
                    let received = data;
                    // Reset when client restarts
                    if received.sequence == 1 {
                        last = 0;
                    }
                    if received.sequence > last {
                        last += 1;
                        info!(
                            "EchoReply: seqnr = {:?}, time = {:?} us",
                            received.sequence,
                            (now.as_micros() as u64) - received.when_us
                        );
                    } else {
                        trace!("Duplicate")
                    }
                    small_trace!(end_echo_reply_received);
                }
                Err(QueuingRecvError::Apex(Error::InvalidConfig)) => {
                    warn!("The queue overflowed");
                }
                Err(QueuingRecvError::Apex(Error::NotAvailable))
                | Err(QueuingRecvError::Apex(Error::TimedOut)) => {
                    debug!("No echo reply available");
                }
                Err(QueuingRecvError::Postcard(e, _)) => {
                    trace!("Failed to decode echo reply: {e:?}");
                }
                Err(e) => {
                    error!("Failed to receive reply: {e:?}");
                }
            }
        }
    }
}

pub struct QueuingPeriodicEchoPartition<
    const ECHO_SIZE: MessageSize,
    const FIFO_DEPTH: MessageRange,
    S,
> where
    S: ApexQueuingPortP4 + 'static,
{
    sender: &'static OnceCell<QueuingPortSender<ECHO_SIZE, FIFO_DEPTH, S>>,
    receiver: &'static OnceCell<QueuingPortReceiver<ECHO_SIZE, FIFO_DEPTH, S>>,
    entry_point_periodic: extern "C" fn(),
    entry_point_aperiodic: extern "C" fn(),
}

impl<const ECHO_SIZE: MessageSize, const FIFO_DEPTH: MessageRange, H> Partition<H>
    for QueuingPeriodicEchoPartition<ECHO_SIZE, FIFO_DEPTH, H>
where
    H: ApexPartitionP4 + ApexProcessP4 + ApexQueuingPortP4,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        trace!("Cold start echo client");

        // Check if configured to use sampling port
        let send_port = ctx
            .create_queuing_port_sender(
                Name::from_str("EchoRequest").unwrap(),
                QueuingDiscipline::FIFO,
                FIFO_DEPTH,
            )
            .unwrap();

        _ = self.sender.set(send_port);

        let receive_port = ctx
            .create_queuing_port_receiver(
                Name::from_str("EchoReply").unwrap(),
                QueuingDiscipline::FIFO,
                FIFO_DEPTH,
            )
            .unwrap();
        _ = self.receiver.set(receive_port);

        // Periodic
        trace!("Creating periodic echo process");
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Normal(Duration::from_secs(1)),
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point_periodic,
            stack_size: 10000,
            base_priority: 5,
            deadline: Deadline::Soft,
            name: Name::from_str("EchoSend").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap();

        // Aperiodic
        trace!("Creating aperiodic echo process");
        ctx.create_process(ProcessAttribute {
            // There can be only one process with normal period
            period: SystemTime::Infinite,
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point_aperiodic,
            stack_size: 10000,
            base_priority: 1,
            deadline: Deadline::Soft,
            name: Name::from_str("EchoReceive").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap()
    }

    fn warm_start(&self, ctx: &mut StartContext<H>) {
        self.cold_start(ctx)
    }
}

impl<const ECHO_SIZE: MessageSize, const FIFO_DEPTH: MessageRange, H>
    QueuingPeriodicEchoPartition<ECHO_SIZE, FIFO_DEPTH, H>
where
    H: ApexQueuingPortP4,
    [u8; ECHO_SIZE as usize]:,
{
    pub fn new(
        sender: &'static OnceCell<QueuingPortSender<ECHO_SIZE, FIFO_DEPTH, H>>,
        receiver: &'static OnceCell<QueuingPortReceiver<ECHO_SIZE, FIFO_DEPTH, H>>,
        entry_point_periodic: extern "C" fn(),
        entry_point_aperiodic: extern "C" fn(),
    ) -> Self {
        QueuingPeriodicEchoPartition::<ECHO_SIZE, FIFO_DEPTH, H> {
            sender,
            receiver,
            entry_point_periodic,
            entry_point_aperiodic,
        }
    }
}
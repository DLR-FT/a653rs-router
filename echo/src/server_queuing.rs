use a653rs::bindings::{ApexPartitionP4, ApexProcessP4, ApexQueuingPortP4};
use a653rs::prelude::*;
use core::str::FromStr;
use core::time::Duration;
use log::{debug, error, info, trace, warn};
use once_cell::unsync::OnceCell;
use small_trace::small_trace;

pub struct EchoServerPartition<const ECHO_SIZE: MessageSize, const RANGE: MessageRange, S>
where
    S: ApexQueuingPortP4 + 'static,
{
    sender: &'static OnceCell<QueuingPortSender<ECHO_SIZE, RANGE, S>>,
    receiver: &'static OnceCell<QueuingPortReceiver<ECHO_SIZE, RANGE, S>>,
    entry_point_aperiodic: extern "C" fn(),
}

impl<const ECHO_SIZE: MessageSize, const RANGE: MessageRange, S>
    EchoServerPartition<ECHO_SIZE, RANGE, S>
where
    S: ApexQueuingPortP4 + 'static,
{
    pub fn new(
        sender: &'static OnceCell<QueuingPortSender<ECHO_SIZE, RANGE, S>>,
        receiver: &'static OnceCell<QueuingPortReceiver<ECHO_SIZE, RANGE, S>>,
        entry_point_aperiodic: extern "C" fn(),
    ) -> Self {
        Self {
            sender,
            receiver,
            entry_point_aperiodic,
        }
    }
}

impl<const ECHO_SIZE: MessageSize, const RANGE: MessageRange, H> Partition<H>
    for EchoServerPartition<ECHO_SIZE, RANGE, H>
where
    H: ApexPartitionP4 + ApexProcessP4 + ApexQueuingPortP4,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        info!("Echo server cold start");
        {
            let recv = ctx
                .create_queuing_port_receiver(
                    Name::from_str("EchoRequest").unwrap(),
                    QueuingDiscipline::Fifo,
                )
                .unwrap();
            _ = self.receiver.set(recv);
        };

        {
            let send = ctx
                .create_queuing_port_sender(
                    Name::from_str("EchoReply").unwrap(),
                    QueuingDiscipline::Fifo,
                )
                .unwrap();
            _ = self.sender.set(send);
        };
        ctx.create_process(ProcessAttribute {
            period: SystemTime::Infinite,
            time_capacity: SystemTime::Infinite,
            entry_point: self.entry_point_aperiodic,
            // TODO make configurable
            stack_size: 20_000,
            base_priority: 5,
            deadline: Deadline::Soft,
            name: Name::from_str("echo_server").unwrap(),
        })
        .unwrap()
        .start()
        .unwrap();
    }

    fn warm_start(&self, ctx: &mut StartContext<H>) {
        self.cold_start(ctx)
    }
}

#[derive(Debug)]
pub struct EchoServerProcess<const ECHO_SIZE: MessageSize, const RANGE: MessageRange>;

const TIMEOUT: SystemTime = SystemTime::Normal(Duration::from_millis(100));

impl<const ECHO_SIZE: MessageSize, const RANGE: MessageRange> EchoServerProcess<ECHO_SIZE, RANGE>
where
    [u8; ECHO_SIZE as usize]:,
{
    pub fn run<H: ApexQueuingPortP4 + ApexTimeP4Ext>(
        send: &mut QueuingPortSender<ECHO_SIZE, RANGE, H>,
        recv: &mut QueuingPortReceiver<ECHO_SIZE, RANGE, H>,
    ) {
        info!("Running echo server");
        let mut buf = [0u8; ECHO_SIZE as usize];
        loop {
            match recv.receive(&mut buf, TIMEOUT) {
                Ok(data) => {
                    small_trace!(begin_echo_request_received);
                    trace!("Received echo request: ${data:?}");
                    if data.is_empty() {
                        trace!("Skipping empty data");
                        continue;
                    }
                    small_trace!(begin_echo_reply_send);
                    match send.send(data, TIMEOUT) {
                        Ok(_) => {
                            trace!("Replied to echo");
                        }
                        Err(err) => {
                            warn!("Failed to reply to echo: {err:?}");
                        }
                    }
                    small_trace!(end_echo_reply_send);
                    small_trace!(end_echo_request_received);
                }
                Err(Error::NotAvailable) | Err(Error::NoAction) | Err(Error::TimedOut) => {
                    trace!("No echo request available yet");
                }
                Err(e) => {
                    error!("Failed to receive echo: ${e:?}");
                }
            }
        }
    }
}

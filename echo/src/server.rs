use a653rs::prelude::*;
use core::str::FromStr;
use core::time::Duration;
use log::{debug, error, info, trace, warn};
use once_cell::unsync::OnceCell;
use small_trace::small_trace;

pub struct EchoServerPartition<const ECHO_SIZE: MessageSize, S>
where
    S: ApexSamplingPortP4 + 'static,
{
    sender: &'static OnceCell<SamplingPortSource<ECHO_SIZE, S>>,
    receiver: &'static OnceCell<SamplingPortDestination<ECHO_SIZE, S>>,
    entry_point_aperiodic: extern "C" fn(),
}

impl<const ECHO_SIZE: MessageSize, S> EchoServerPartition<ECHO_SIZE, S>
where
    S: ApexSamplingPortP4 + 'static,
{
    pub fn new(
        sender: &'static OnceCell<SamplingPortSource<ECHO_SIZE, S>>,
        receiver: &'static OnceCell<SamplingPortDestination<ECHO_SIZE, S>>,
        entry_point_aperiodic: extern "C" fn(),
    ) -> Self {
        Self {
            sender,
            receiver,
            entry_point_aperiodic,
        }
    }
}

impl<const ECHO_SIZE: MessageSize, H> Partition<H> for EchoServerPartition<ECHO_SIZE, H>
where
    H: ApexPartitionP4 + ApexProcessP4 + ApexSamplingPortP4,
{
    fn cold_start(&self, ctx: &mut StartContext<H>) {
        info!("Echo server cold start");
        {
            let recv = ctx
                .create_sampling_port_destination(
                    Name::from_str("EchoRequest").unwrap(),
                    Duration::from_secs(10),
                )
                .unwrap();
            _ = self.receiver.set(recv);
        };

        {
            let send = ctx
                .create_sampling_port_source(Name::from_str("EchoReply").unwrap())
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
pub struct EchoServerProcess<const ECHO_SIZE: MessageSize>;

impl<const ECHO_SIZE: MessageSize> EchoServerProcess<ECHO_SIZE>
where
    [u8; ECHO_SIZE as usize]:,
{
    pub fn run<H: ApexSamplingPortP4 + ApexTimeP4Ext>(
        send: &mut SamplingPortSource<ECHO_SIZE, H>,
        recv: &mut SamplingPortDestination<ECHO_SIZE, H>,
    ) {
        info!("Running echo server");
        let mut buf = [0u8; ECHO_SIZE as usize];
        loop {
            match recv.receive(&mut buf) {
                Ok((val, data)) => {
                    small_trace!(begin_echo_request_received);
                    if data.is_empty() {
                        trace!("Skipping empty data");
                        continue;
                    }
                    debug!("Received echo request: {data:?}");
                    if val == Validity::Valid {
                        small_trace!(begin_echo_reply_send);
                        match send.send(data) {
                            Ok(_) => {
                                info!("Replied to echo");
                            }
                            Err(err) => {
                                error!("Failed to reply to echo: {err:?}");
                            }
                        }
                        small_trace!(end_echo_reply_send);
                    } else {
                        warn!("Ignoring invalid data");
                    }
                    small_trace!(end_echo_request_received);
                }
                Err(Error::NotAvailable) | Err(Error::NoAction) => {
                    warn!("No echo request available yet");
                }
                Err(e) => {
                    error!("Failed to receive echo: ${e:?}");
                }
            }
        }
    }
}

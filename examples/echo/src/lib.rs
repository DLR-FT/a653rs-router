#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use a653rs::bindings::ApexQueuingPortP4;
use a653rs::bindings::ApexSamplingPortP4;
use a653rs::prelude::*;
use a653rs_postcard::prelude::*;
use a653rs_postcard::{
    prelude::QueuingPortReceiverExt, prelude::QueuingPortSenderExt, prelude::QueuingRecvError,
};

use core::time::Duration;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use small_trace::small_trace;

pub const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

const TIMEOUT: SystemTime = SystemTime::Normal(Duration::from_millis(100));

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
/// Echo message
pub struct EchoMessage {
    /// A sequence number.
    pub sequence: u32,

    /// The time at which the message has been created.
    pub when_us: u64,
}

pub fn run_echo_queuing_receiver<H: ApexQueuingPortP4 + ApexTimeP4Ext>(
    port: &QueuingPortReceiver<1000, 10, H>,
) {
    let mut last = 0;
    loop {
        trace!("Running echo client aperiodic process");

        let result = port.recv_type::<EchoMessage>(SystemTime::Normal(Duration::from_millis(100)));

        let time = <H as ApexTimeP4Ext>::get_time();
        let now = match time {
            SystemTime::Infinite => {
                continue;
            }
            SystemTime::Normal(now) => now,
        };

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
                warn!("Failed to receive reply: {e:?}");
            }
        }
    }
}

pub fn run_echo_queuing_sender<H: ApexQueuingPortP4 + ApexTimeP4Ext>(
    port: &QueuingPortSender<1000, 10, H>,
) {
    info!("Running echo client periodic process");
    let mut i: u32 = 0;
    loop {
        let time = <H as ApexTimeP4Ext>::get_time();
        let now = match time {
            SystemTime::Infinite => {
                continue;
            }
            SystemTime::Normal(now) => now,
        };
        i += 1;
        let data = EchoMessage {
            sequence: i,
            when_us: now.as_micros() as u64,
        };
        small_trace!(begin_echo_request_send);
        let result = port.send_type(data, SystemTime::Normal(Duration::from_micros(10)));
        small_trace!(end_echo_request_send);
        match result {
            Ok(_) => {
                info!(
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
        <H as ApexTimeP4Ext>::periodic_wait().unwrap()
    }
}

pub fn run_server_queuing_main<H: ApexQueuingPortP4 + ApexTimeP4Ext>(
    port_out: &QueuingPortSender<1000, 10, H>,
    port_in: &QueuingPortReceiver<1000, 10, H>,
) {
    info!("Running echo server");
    let mut buf = [0u8; 1000 as usize];
    loop {
        match port_in.receive(&mut buf, TIMEOUT) {
            Ok(data) => {
                small_trace!(begin_echo_request_received);
                trace!("Received echo request: ${data:?}");
                if data.is_empty() {
                    trace!("Skipping empty data");
                    continue;
                }
                small_trace!(begin_echo_reply_send);
                match port_out.send(data, TIMEOUT) {
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

pub fn run_echo_sampling_receiver<H: ApexSamplingPortP4 + ApexTimeP4Ext>(
    port: &SamplingPortDestination<1000, H>,
) {
    info!("Running echo client aperiodic process");
    let mut last = 0;
    loop {
        trace!("Receiving from port");

        let result = port.recv_type::<EchoMessage>();
        match result {
            Ok(data) => {
                small_trace!(begin_echo_reply_received);
                trace!("Received reply: {data:?}");
                let (_, received) = data;
                // Reset when client restarts
                if received.sequence == 1 {
                    last = 1;
                }
                if received.sequence > last {
                    last += 1;
                    let time = <H as ApexTimeP4Ext>::get_time();
                    match time {
                        SystemTime::Normal(now) => {
                            info!(
                                "EchoReply: seqnr = {:?}, time = {:?} us",
                                received.sequence,
                                (now.as_micros() as u64) - received.when_us
                            );
                        }
                        _ => {
                            warn!("Failed to get time from hypervisor");
                        }
                    }
                } else {
                    trace!("Duplicate")
                }
                small_trace!(end_echo_reply_received);
            }
            Err(SamplingRecvError::Apex(Error::NotAvailable)) => {
                debug!("No echo reply available");
            }
            Err(SamplingRecvError::Postcard(e, _, _)) => {
                trace!("Failed to decode echo reply: {e:?}");
            }
            _ => {
                debug!("Failed to receive echo reply");
            }
        }
    }
}

pub fn run_echo_sampling_sender<H: ApexSamplingPortP4 + ApexTimeP4Ext>(
    port: &SamplingPortSource<1000, H>,
) {
    info!("Running echo client periodic process");
    let mut i: u32 = 0;
    loop {
        i += 1;
        let time = <H as ApexTimeP4Ext>::get_time();
        match time {
            SystemTime::Normal(now) => {
                let data = EchoMessage {
                    sequence: i,
                    when_us: now.as_micros() as u64,
                };
                small_trace!(begin_echo_request_send);
                let result = port.send_type(data);
                small_trace!(end_echo_request_send);
                match result {
                    Ok(_) => {
                        info!(
                            "EchoRequest: seqnr = {:?}, time = {:?} us",
                            data.sequence, data.when_us
                        );
                    }
                    Err(_) => {
                        error!("Failed to send EchoRequest");
                    }
                }
            }
            _ => {
                warn!("Failed to get time from hypervisor")
            }
        }
        trace!("Going to sleep ...");
        <H as ApexTimeP4Ext>::periodic_wait().unwrap();
    }
}

pub fn run_server_sampling_main<H: ApexSamplingPortP4 + ApexTimeP4Ext>(
    port_out: &SamplingPortSource<1000, H>,
    port_in: &SamplingPortDestination<1000, H>,
) {
    info!("Running echo server");
    let mut buf = [0u8; 1000_usize];
    loop {
        match port_in.receive(&mut buf) {
            Ok((val, data)) => {
                small_trace!(begin_echo_request_received);
                if data.is_empty() {
                    trace!("Skipping empty data");
                    continue;
                }
                debug!("Received echo request: {data:?}");
                if val == Validity::Valid {
                    small_trace!(begin_echo_reply_send);
                    match port_out.send(data) {
                        Ok(_) => {
                            debug!("Replied to echo");
                        }
                        Err(err) => {
                            error!("Failed to reply to echo: {err:?}");
                        }
                    }
                    small_trace!(end_echo_reply_send);
                } else {
                    debug!("Ignoring invalid data");
                }
                small_trace!(end_echo_request_received);
            }
            Err(Error::NotAvailable) | Err(Error::NoAction) => {
                trace!("No echo request available yet");
            }
            Err(e) => {
                error!("Failed to receive echo: ${e:?}");
            }
        }
    }
}

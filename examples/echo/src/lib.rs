//! Echo sender and responder
//!
//! This crate contains functions for running an echo sender and responder that
//! works similarly to the `ping` command. Instead of sending ICMP probes, it
//! transmits a custom probe message format on ARINC 653 sampling or
//! queuing ports. The end-result is similar -- the round-trip-times for all
//! successfully received packets are logged along with their sequence numbers.
//!
//! The functionality contained within this crate is for use by the more
//! platform-specific crates `echo-linux` and `echo-xng`, which contain code for
//! storing the hypervisor-specific data-types needed by a653rs. To not require
//! more than one compilation unit per target to cover all possible
//! configurations, some run-time configuration is required.
//! Possible configurations are combinations of sampling / queuing channels and
//! client / server modes. There are two entry-functions which check which kinds
//! of hypervisor-ports are available and use this information to determine
//! which configuration is in use.
//!
//! - [`cold_start_sampling_queuing`]: Checks if it can create queuing ports
//!   `EchoReceiv` and `EchoSend` to check if running as a client and
//!   `SEchoReceiv` and `SEchoSend` to check if it is running as a server. If
//!   this did not succeed, it will try the same but using sampling ports.
//! - [`cold_start_sampling`]: Does the same as `cold_start_sampling_queuing`,
//!   but without checking for queuing ports first. This function exists for
//!   hypervisors that do not implement queuing ports.

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
use core::fmt::Debug;
use core::str::FromStr;
use core::time::Duration;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use small_trace::small_trace;

pub const ECHO_SIZE: MessageSize = 1000;
pub const ECHO_QUEUE_SIZE: MessageRange = 10;

const VALIDITY: Duration = Duration::from_secs(2);
const STACK_SIZE: u32 = 20_000;

const CLIENT_PERIODIC_PROCESS_NAME: &str = "echo_client_sender";
const CLIENT_SEND_BASE_PRIORITY: i32 = 5;
const CLIENT_SEND_PERIOD: SystemTime = SystemTime::Normal(Duration::from_secs(1));
const CLIENT_SEND_TIME_CAPACITY: SystemTime = SystemTime::Infinite;
const CLIENT_SEND_DEADLINE: Deadline = Deadline::Soft;

const CLIENT_APERIODIC_PROCESS_NAME: &str = "echo_client_receiver";
const CLIENT_RECEIVE_BASE_PRIORITY: i32 = 1;
const CLIENT_RECEIVE_PERIOD: SystemTime = SystemTime::Infinite;
const CLIENT_RECEIVE_TIME_CAPACITY: SystemTime = SystemTime::Infinite;
const CLIENT_RECEIVE_DEADLINE: Deadline = Deadline::Soft;

const SERVER_APERIODIC_PROCESS_NAME: &str = "echo_server";
const SERVER_RECEIVE_BASE_PRIORITY: i32 = 5;
const SERVER_RECEIVE_PERIOD: SystemTime = SystemTime::Infinite;
const SERVER_RECEIVE_TIME_CAPACITY: SystemTime = SystemTime::Infinite;
const SERVER_RECEIVE_DEADLINE: Deadline = Deadline::Soft;

const CLIENT_RECEIVER_PORT: &str = "EchoReceive";
const CLIENT_SENDER_PORT: &str = "EchoSend";

const SERVER_RECEIVER_PORT: &str = "SEchoReceive";
const SERVER_SENDER_PORT: &str = "SEchoSend";

const RECEIVE_TIMEOUT: SystemTime = SystemTime::Normal(Duration::from_millis(100));

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum EchoStation {
    Client,
    Server,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum EchoMode {
    Sampling,
    Queueing,
}

#[derive(Debug, Copy, Clone)]
struct EchoConfig {
    station: EchoStation,
    mode: EchoMode,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
/// Echo message
struct EchoMessage {
    /// A sequence number.
    sequence: u32,

    /// The time at which the message has been created.
    when_us: u64,
}

pub fn run_echo_queuing_receiver<const M: u32, const L: u32, H: ApexQueuingPortP4 + ApexTimeP4Ext>(
    port: &QueuingPortReceiver<M, L, H>,
) where
    [u8; M as usize]:,
    [u8; L as usize]:,
{
    let mut last = 0;
    loop {
        trace!("Running echo client aperiodic process");

        let result = port.recv_type::<EchoMessage>(RECEIVE_TIMEOUT);

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

pub fn run_echo_queuing_sender<const M: u32, const L: u32, H: ApexQueuingPortP4 + ApexTimeP4Ext>(
    port: &QueuingPortSender<M, L, H>,
) where
    [u8; M as usize]:,
    [u8; L as usize]:,
{
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

pub fn run_echo_queuing_server<const M: u32, H: ApexQueuingPortP4 + ApexTimeP4Ext>(
    port_out: &QueuingPortSender<M, ECHO_QUEUE_SIZE, H>,
    port_in: &QueuingPortReceiver<M, ECHO_QUEUE_SIZE, H>,
) where
    [u8; M as usize]:,
{
    info!("Running echo server");
    let mut buf = [0u8; M as usize];
    loop {
        match port_in.receive(&mut buf, RECEIVE_TIMEOUT) {
            Ok(data) => {
                small_trace!(begin_echo_request_received);
                trace!("Received echo request: ${data:?}");
                if data.is_empty() {
                    trace!("Skipping empty data");
                    continue;
                }
                small_trace!(begin_echo_reply_send);
                match port_out.send(data, RECEIVE_TIMEOUT) {
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

pub fn run_echo_sampling_receiver<const M: u32, H: ApexSamplingPortP4 + ApexTimeP4Ext>(
    port: &SamplingPortDestination<M, H>,
) where
    [u8; M as usize]:,
{
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

pub fn run_echo_sampling_sender<const M: u32, H: ApexSamplingPortP4 + ApexTimeP4Ext>(
    port: &SamplingPortSource<M, H>,
) where
    [u8; M as usize]:,
{
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

pub fn run_echo_sampling_server<const M: u32, H: ApexSamplingPortP4 + ApexTimeP4Ext>(
    port_out: &SamplingPortSource<M, H>,
    port_in: &SamplingPortDestination<M, H>,
) where
    [u8; M as usize]:,
{
    info!("Running echo server");
    let mut buf = [0u8; M as usize];
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

pub struct EchoEntryFunctions {
    pub client_send_sampling: extern "C" fn(),
    pub client_receive_sampling: extern "C" fn(),
    pub server_sampling: extern "C" fn(),
    pub client_send_queuing: extern "C" fn(),
    pub client_receive_queuing: extern "C" fn(),
    pub server_queuing: extern "C" fn(),
}

pub fn cold_start_sampling_queuing<H>(
    ctx: &mut StartContext<H>,
    queuing_sender: &mut Option<QueuingPortSender<ECHO_SIZE, ECHO_QUEUE_SIZE, H>>,
    queuing_receiver: &mut Option<QueuingPortReceiver<ECHO_SIZE, ECHO_QUEUE_SIZE, H>>,
    sampling_sender: &mut Option<SamplingPortSource<ECHO_SIZE, H>>,
    sampling_receiver: &mut Option<SamplingPortDestination<ECHO_SIZE, H>>,
    entries: &EchoEntryFunctions,
) where
    H: ApexSamplingPortP4Ext + ApexQueuingPortP4Ext + ApexPartitionP4 + ApexProcessP4 + Debug,
{
    let cfg = if let Ok(station) = try_init_queuing(ctx, queuing_sender, queuing_receiver) {
        EchoConfig {
            station,
            mode: EchoMode::Queueing,
        }
    } else {
        info!("Initialization of queuing ports did not succeed, assuming sampling ports are configured.");
        let station = try_init_sampling(ctx, sampling_sender, sampling_receiver)
            .expect("Failed to initialize either queuing or sampling ports.");
        EchoConfig {
            station,
            mode: EchoMode::Sampling,
        }
    };
    init_processes(ctx, &cfg, entries);
}

pub fn cold_start_sampling<H>(
    ctx: &mut StartContext<H>,
    sender: &mut Option<SamplingPortSource<ECHO_SIZE, H>>,
    receiver: &mut Option<SamplingPortDestination<ECHO_SIZE, H>>,
    entries: &EchoEntryFunctions,
) where
    H: ApexSamplingPortP4Ext + ApexPartitionP4 + ApexProcessP4 + Debug,
{
    let station =
        try_init_sampling(ctx, sender, receiver).expect("Failed to initialize sampling ports");
    let cfg = EchoConfig {
        station,
        mode: EchoMode::Sampling,
    };
    init_processes(ctx, &cfg, entries);
}

fn init_processes<H>(ctx: &mut StartContext<H>, cfg: &EchoConfig, entries: &EchoEntryFunctions)
where
    H: ApexProcessP4 + Debug,
{
    match cfg.station {
        EchoStation::Client => match cfg.mode {
            EchoMode::Sampling => init_client(
                ctx,
                entries.client_send_sampling,
                entries.client_receive_sampling,
            ),
            EchoMode::Queueing => init_client(
                ctx,
                entries.client_send_queuing,
                entries.client_receive_queuing,
            ),
        },
        EchoStation::Server => match cfg.mode {
            EchoMode::Sampling => init_server(ctx, entries.server_sampling),
            EchoMode::Queueing => init_server(ctx, entries.server_queuing),
        },
    }
}

fn try_init_sampling<H>(
    ctx: &mut StartContext<H>,
    sender: &mut Option<SamplingPortSource<ECHO_SIZE, H>>,
    receiver: &mut Option<SamplingPortDestination<ECHO_SIZE, H>>,
) -> Result<EchoStation, Error>
where
    H: ApexSamplingPortP4Ext + Debug,
{
    if let Ok(port) = ctx.create_sampling_port_source(Name::from_str(CLIENT_SENDER_PORT).unwrap()) {
        _ = sender.insert(port);
        _ = receiver.insert(ctx.create_sampling_port_destination(
            Name::from_str(CLIENT_RECEIVER_PORT).unwrap(),
            VALIDITY,
        )?);
        Ok(EchoStation::Client)
    } else {
        _ = sender
            .insert(ctx.create_sampling_port_source(Name::from_str(SERVER_SENDER_PORT).unwrap())?);
        _ = receiver.insert(ctx.create_sampling_port_destination(
            Name::from_str(SERVER_RECEIVER_PORT).unwrap(),
            VALIDITY,
        )?);
        Ok(EchoStation::Server)
    }
}

fn try_init_queuing<H>(
    ctx: &mut StartContext<H>,
    sender: &mut Option<QueuingPortSender<ECHO_SIZE, ECHO_QUEUE_SIZE, H>>,
    receiver: &mut Option<QueuingPortReceiver<ECHO_SIZE, ECHO_QUEUE_SIZE, H>>,
) -> Result<EchoStation, Error>
where
    H: ApexQueuingPortP4Ext + Debug,
{
    if let Ok(port) = ctx.create_queuing_port_sender(
        Name::from_str(CLIENT_SENDER_PORT).unwrap(),
        QueuingDiscipline::Fifo,
    ) {
        _ = sender.insert(port);
        _ = receiver.insert(ctx.create_queuing_port_receiver(
            Name::from_str(CLIENT_RECEIVER_PORT).unwrap(),
            QueuingDiscipline::Fifo,
        )?);
        Ok(EchoStation::Client)
    } else {
        _ = sender.insert(ctx.create_queuing_port_sender(
            Name::from_str(SERVER_SENDER_PORT).unwrap(),
            QueuingDiscipline::Fifo,
        )?);
        _ = receiver.insert(ctx.create_queuing_port_receiver(
            Name::from_str(SERVER_RECEIVER_PORT).unwrap(),
            QueuingDiscipline::Fifo,
        )?);
        Ok(EchoStation::Client)
    }
}

fn init_client<H>(
    ctx: &mut StartContext<H>,
    echo_client_send: extern "C" fn(),
    echo_client_receive: extern "C" fn(),
) where
    H: ApexProcessP4 + Debug,
{
    ctx.create_process(ProcessAttribute {
        period: CLIENT_SEND_PERIOD,
        time_capacity: CLIENT_SEND_TIME_CAPACITY,
        entry_point: echo_client_send,
        stack_size: STACK_SIZE,
        base_priority: CLIENT_SEND_BASE_PRIORITY,
        deadline: CLIENT_SEND_DEADLINE,
        name: Name::from_str(CLIENT_PERIODIC_PROCESS_NAME).unwrap(),
    })
    .unwrap()
    .start()
    .unwrap();
    ctx.create_process(ProcessAttribute {
        period: CLIENT_RECEIVE_PERIOD,
        time_capacity: CLIENT_RECEIVE_TIME_CAPACITY,
        entry_point: echo_client_receive,
        stack_size: STACK_SIZE,
        base_priority: CLIENT_RECEIVE_BASE_PRIORITY,
        deadline: CLIENT_RECEIVE_DEADLINE,
        name: Name::from_str(CLIENT_APERIODIC_PROCESS_NAME).unwrap(),
    })
    .unwrap()
    .start()
    .unwrap();
}

fn init_server<H>(ctx: &mut StartContext<H>, echo_server: extern "C" fn())
where
    H: ApexProcessP4 + Debug,
{
    ctx.create_process(ProcessAttribute {
        period: SERVER_RECEIVE_PERIOD,
        time_capacity: SERVER_RECEIVE_TIME_CAPACITY,
        entry_point: echo_server,
        stack_size: STACK_SIZE,
        base_priority: SERVER_RECEIVE_BASE_PRIORITY,
        deadline: SERVER_RECEIVE_DEADLINE,
        name: Name::from_str(SERVER_APERIODIC_PROCESS_NAME).unwrap(),
    })
    .unwrap()
    .start()
    .unwrap();
}

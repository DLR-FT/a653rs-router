//! Traffic shapers

use crate::error::Error;
use bytesize::ByteSize;
use core::time::Duration;
use heapless::Vec;

// TODO TrafficClass -> bandwidth_fraction = idle_slope / port_transmit_rate
// Make sure total port_transmit_rate is not exceeded
// Use up remaining bandwidth of a frame in order of priorities of streams
// credit must be limited to 0 if no messages are waiting
// Will probably have to store queues for each Stream (with capacity 0 for sampling ports?)
// Idea: Read from port to check if it has queued messages / valid message -> queue depth

/// Credit-based shaper roughly after IEEE 802.1Qav Credit-Based Shaper
///
/// The shaper decides if a queued message should be transmitted or not.
/// A queued message will be transmitted, only if the credit of the stream that emitted the message is non-negative (0 is ok).
///
/// A stream corresponds to a virtual link.
/// Individual streams may only transmit if their credit is not negative.
/// Time is divided into frames of the size of each major frame window.
/// All links are assumed to compete for the same network resource.
/// The credit of a stream is replenished during the major frame.
///     idle_slope = reserved_bytes / major_frame_length
/// The credit of a stream is consumed for each time the network partition is scheduled and transmits messages from the stream
///     send_slope = transmitted_bytes / network_partition_frame_length
/// The maximum total credit of all streams combined can not be larger than the credit that can be replenished during a major
/// frame (the bytes the network partition may transmit during one major frame) or the network might be overuttilized.
/// For this the maximum credit of each partition is limited to the configured maximum data rate during a major frame that each
/// partition may emit and it does not increased boyond that.
/// It is assumed that the network partition is scheduled once per major frame.
/// TODO must be able to inspect transmission events on hardware
/// TODO const params seems very unwieldy. Does not allow for different frame sizes per queue.

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct QueueId(u32);

impl From<QueueId> for u32 {
    fn from(val: QueueId) -> Self {
        val.0
    }
}

impl From<u32> for QueueId {
    fn from(val: u32) -> Self {
        QueueId(val)
    }
}

/// A transmission of the network layer.
///
/// The transmission occurs for a frame from a queue designated by `queue_id`, lasts for `duration` and transmits `bits`.
#[derive(Debug, Default)]
pub struct Transmission {
    /// ID of the queue from which bits have been transmitted.
    queue_id: QueueId,

    /// The time it took to transmit the bits.
    duration: Duration,

    /// The amount of bits that were transmitted.
    bits: u64,
}

impl Transmission {
    /// Creates a new transmission.
    pub fn new(queue: QueueId, duration: Duration, length: ByteSize) -> Self {
        Self {
            queue_id: queue,
            duration,
            bits: length.as_u64() * 8,
        }
    }

    /// Updates the transmission with the actual duration of the transmission.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

// TODO shape collection of queues
// get stati
// find next queue
// transmit
// credit queues
// return

/// A traffic shaper.
pub trait Shaper {
    /// Requests that the shaper allows the queue to perform a transmission.
    fn request_transmission(&mut self, transmission: &Transmission) -> Result<(), Error>;
    /// Notifies the shaper, that a transmission took place.
    /// Returns the number of consumed bits.
    fn record_transmission(&mut self, transmission: &Transmission) -> Result<(), Error>;
    /// Gets the id of the queue that may transmit the next frame.
    fn next_queue(&mut self) -> Option<QueueId>;
}

/// A credit-based shaper similar to 802.1Qav.
#[derive(Debug)]
pub struct CreditBasedShaper<const NUM_QUEUES: usize> {
    port_transmit_rate: u64,
    free_bandwidth: u64,
    queues: Vec<QueueStatus, NUM_QUEUES>,
}

impl<const NUM_QUEUES: usize> CreditBasedShaper<NUM_QUEUES> {
    /// Creates a new credit-based shaper.
    pub fn new(port_transmit_rate: ByteSize) -> Self {
        let port_transmit_rate = port_transmit_rate.as_u64() * 8;
        Self {
            port_transmit_rate,
            free_bandwidth: port_transmit_rate,
            queues: Vec::default(),
        }
    }

    /// Adds a new queue to the shaper that uses a share of the bandwidth specified by `share`.
    pub fn add_queue(&mut self, share: ByteSize) -> Result<QueueId, Error> {
        let share = share.as_u64() * 8;
        let id = QueueId::from(self.queues.len() as u32);
        let q = if self.free_bandwidth >= share {
            self.free_bandwidth -= share;
            Ok(QueueStatus::new(id, share, self.port_transmit_rate))
        } else {
            Err(Error::Unknown) // TODO
        }?;
        if let Err(_) = self.queues.push(q) {
            return Err(Error::Unknown); // TODO
        }

        return Ok(id);
    }
}

impl<const NUM_QUEUES: usize> Shaper for CreditBasedShaper<NUM_QUEUES> {
    fn request_transmission(&mut self, transmission: &Transmission) -> Result<(), Error> {
        let q_id: usize = transmission.queue_id.0 as usize;
        let q = self.queues.get_mut(q_id).ok_or(Error::InvalidData)?; // TODO better error
        q.submit(transmission.bits);
        Ok(())
    }

    fn record_transmission(&mut self, transmission: &Transmission) -> Result<(), Error> {
        let mut consumed = false;
        for q in self.queues.iter_mut() {
            if q.id == transmission.queue_id {
                q.transmit = false;
                _ = q.consume(&transmission.bits, &transmission.duration)?;
                consumed = true;
            } else {
                _ = q.restore(&transmission.duration)?;
            }
        }
        if consumed {
            Ok(())
        } else {
            Err(Error::InvalidData)
        }
    }

    /// Determines which queue is next.
    /// - queues: The available queues sorted by their priority.
    /// It is assumed that each queue services frames of a limited size so there is a lo_credit for each queue.
    fn next_queue(&mut self) -> Option<QueueId> {
        for q in self.queues.iter_mut() {
            if q.transmit_allowed() {
                q.transmit = true;
                return Some(q.id);
            }
        }
        None
    }
}

/// A stream that is managed by the shaper.
#[derive(Debug, Default, PartialEq, Eq)]
struct QueueStatus {
    /// Queue id.
    id: QueueId,

    /// Idle slope in bits per second.
    ///
    /// The credit increases by this rate while the queue is not allowed to transmit.
    /// The idle slope also determines the bandwidth share of the queue.
    ///
    /// bandwidth_fraction = idle_slope / port_transmit_rate
    ///
    idle_slope: u64,

    /// Current credit of the queue.
    credit: i128,

    /// True if this queue is currently transmitting.
    transmit: bool,

    /// The rate in bits per second by which the credit of this queue is diminished
    /// while the queue is transmitting frames.
    ///
    /// send_slope = idle_slope - port_transmit_rate
    send_slope: u64,

    /// Number of queued bit.
    backlog: u64,
}

impl QueueStatus {
    /// Creates a new CreditQueue.
    fn new(id: QueueId, idle_slope: u64, port_transmit_rate: u64) -> Self {
        Self {
            id,
            idle_slope,
            credit: 0,
            transmit: false,
            send_slope: port_transmit_rate - idle_slope,
            backlog: 0,
        }
    }

    /// If the queue is allowed to begin a transmission of a frame.
    /// True if the queue has credit equal to or greater than 0.
    /// If the queue has no waiting frames, all remaining credit is withdrawn until 0.
    fn transmit_allowed(&self) -> bool {
        self.credit >= 0
    }

    /// Consumes credit from the queue.
    fn consume(&mut self, bits: &u64, duration: &Duration) -> Result<u64, Error> {
        if !self.transmit_allowed() {
            return Err(Error::TransmitNotAllowed);
        }
        let consumed = (self.send_slope * (duration.as_millis() as u64)) / 1000;
        self.credit = self.credit - (consumed as i128);
        self.backlog -= bits;
        Ok(consumed)
    }

    /// Increases the credit, while the queue was blocked by the port transmitting conflicting
    /// traffic or a higher priority queue has queued frames.
    fn restore(&mut self, time: &Duration) -> Result<u64, Error> {
        if self.transmit_allowed() && self.backlog > 0 {
            let credited_bytes = self.idle_slope * (time.as_millis() as u64) / 1000;
            self.credit += credited_bytes as i128;
            Ok(credited_bytes)
        } else {
            if self.backlog == 0 && !self.transmit {
                self.credit = 0;
            }
            Ok(0)
        }
    }

    fn submit(&mut self, bits: u64) {
        self.backlog += bits;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_credit_negative_then_no_transmission_allowed() {
        let queue = QueueStatus {
            id: QueueId(0),
            credit: -10,
            idle_slope: 10,
            transmit: false,
            send_slope: 20,
            backlog: 0,
        };

        assert_eq!(false, queue.transmit_allowed());
    }

    #[test]
    fn given_credit_not_negative_then_transmission_allowed() {
        let queue = QueueStatus {
            id: QueueId(0),
            credit: 10000,
            idle_slope: 10,
            transmit: false,
            send_slope: 20,
            backlog: 0,
        };
        assert_eq!(true, queue.transmit_allowed());
    }

    #[test]
    fn given_no_backlog_when_restore_then_credit_zero() {
        let mut q = QueueStatus {
            id: QueueId(0),
            credit: 10000,
            idle_slope: 10,
            transmit: false,
            send_slope: 20,
            backlog: 0,
        };
        assert!(q.restore(&Duration::from_millis(10)).is_ok());
        assert_eq!(0, q.credit);
    }

    #[test]
    fn given_credit_and_transmit_and_backlog_zero_when_restore_then_credit_unchanged() {
        let credit: i128 = 10_000;
        let mut q = QueueStatus {
            id: QueueId(0),
            credit,
            idle_slope: 10,
            transmit: true,
            send_slope: 20,
            backlog: 0,
        };
        assert!(q.restore(&Duration::from_millis(10)).is_ok());
        assert_eq!(credit, q.credit);
    }

    #[test]
    fn given_not_enough_remaining_bandwidth_when_queue_added_then_error() {
        let mut s = CreditBasedShaper::<2>::new(ByteSize::kb(128));
        assert!(s.add_queue(ByteSize::kb(90)).is_ok());
        assert!(s.add_queue(ByteSize::kb(90)).is_err());
    }

    #[test]
    fn given_two_queues_high_credit_and_backlog_when_both_transmit_then_bandwidth_usage_below_limit(
    ) {
        const BANDWIDTH_LIMIT: ByteSize = ByteSize::kb(100);
        let mut s = CreditBasedShaper::<2>::new(BANDWIDTH_LIMIT);
        let q1 = s.add_queue(ByteSize::kb(60)).unwrap();
        let q2 = s.add_queue(ByteSize::kb(40)).unwrap();

        const MTU_Q1: ByteSize = ByteSize::kb(7);
        const MTU_Q2: ByteSize = ByteSize::kb(4);

        let transmissions = [
            Transmission::new(q1, Duration::ZERO, MTU_Q1),
            Transmission::new(q2, Duration::ZERO, MTU_Q2),
            Transmission::new(q2, Duration::ZERO, MTU_Q2),
            Transmission::new(q1, Duration::ZERO, MTU_Q1),
            Transmission::new(q1, Duration::ZERO, MTU_Q1),
            Transmission::new(q2, Duration::ZERO, MTU_Q2),
            Transmission::new(q1, Duration::ZERO, MTU_Q1),
            Transmission::new(q2, Duration::ZERO, MTU_Q2),
            Transmission::new(q2, Duration::ZERO, MTU_Q2),
            Transmission::new(q1, Duration::ZERO, MTU_Q1),
            Transmission::new(q1, Duration::ZERO, MTU_Q1),
            Transmission::new(q2, Duration::ZERO, MTU_Q2),
        ];

        for t in transmissions.iter() {
            s.request_transmission(t).unwrap();
        }

        // This is how long MTU_Q1/2 will take in the test
        const DURATION_Q1: Duration = Duration::from_millis(70); // min 70 ms
        const DURATION_Q2: Duration = Duration::from_millis(50); // min 40 ms really should take

        let mut total_byte: u64 = 0;
        let mut total_time: Duration = Duration::ZERO;

        // TODO should probably be an iterator
        while let Some(next_q) = s.next_queue() {
            // ... transmit
            if next_q == q1 {
                let t = Transmission::new(q1, DURATION_Q1, MTU_Q1);
                s.record_transmission(&t).unwrap();
                total_byte += MTU_Q1.as_u64();
                total_time += DURATION_Q1;
            } else {
                let t = Transmission::new(q2, DURATION_Q2, MTU_Q2);
                s.record_transmission(&t).unwrap();
                total_byte += MTU_Q2.as_u64();
                total_time += DURATION_Q2;
            }
        }

        let total_time = total_time.as_millis();
        let byte_per_second = ((total_byte as u128) * 1000) / total_time;
        let limit = BANDWIDTH_LIMIT.as_u64() as u128;
        assert!(
                byte_per_second <= limit,
                "Recorded rate: {byte_per_second}, Limit: {limit}, Recorded bytes: {total_byte}, Recorded time: {total_time}"
            );
    }
}

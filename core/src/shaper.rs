//! Traffic shapers

use crate::error::Error;
use crate::types::DataRate;
use core::{fmt::Debug, time::Duration};
use heapless::Vec;
use log::{error, trace};

/// The id of a queue that is managed by the shaper.
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

impl core::fmt::Display for QueueId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A transmission of the network layer.
///
/// The transmission occurs for a frame from a queue designated by `queue_id`, lasts for `duration` and transmits `bits`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Transmission {
    /// The queue that performed or requested the transmission.
    queue: QueueId,

    /// The amount of bits of the encoded message the interface will transmit.
    encoded: u64,
}

impl Transmission {
    /// Creates a new transmission.
    pub(crate) fn new(queue: QueueId, encoded: usize) -> Self {
        Self {
            queue,
            encoded: encoded as u64 * 8,
        }
    }
}

/// A traffic shaper.
pub trait Shaper: Debug {
    /// Requests that the shaper allows the queue to perform a transmission.
    fn request_transmission(&mut self, transmission: &Transmission) -> Result<(), Error>;

    /// Notifies the shaper, that a transmission took place.
    /// Returns the number of consumed bits.
    fn record_transmission(&mut self, transmission: &Transmission) -> Result<(), Error>;

    /// Restores credit to all queues.
    /// Should be called if no transmissions were recorded during a timeframe of length restore.
    fn restore_all(&mut self, restore: Duration) -> Result<(), Error>;

    /// Gets the id of the queue that may transmit the next frame.
    fn next_queue(&mut self) -> Option<QueueId>;

    /// Gets the length of the backlog of a queue.
    fn get_backlog(&self, queue: &QueueId) -> Option<u64>;

    /// Adds a queue with a bandwidth share.
    fn add_queue(&mut self, share: DataRate) -> Option<QueueId>;
}

/// A credit-based shaper similar to 802.1Qav.
#[derive(Debug)]
pub struct CreditBasedShaper<const QUEUES: usize> {
    port_bandwidth: DataRate,
    free_bandwidth: DataRate,
    queues: Vec<QueueStatus, QUEUES>,
}

impl<const QUEUES: usize> CreditBasedShaper<QUEUES> {
    /// Creates a new credit-based shaper.
    pub fn new(port_bandwidth: DataRate) -> Self {
        Self {
            port_bandwidth,
            free_bandwidth: port_bandwidth,
            queues: Vec::default(),
        }
    }
}

impl<const NUM_QUEUES: usize> Shaper for CreditBasedShaper<NUM_QUEUES> {
    fn add_queue(&mut self, share: DataRate) -> Option<QueueId> {
        let free = self.free_bandwidth.as_u64();
        let share = share.as_u64();
        if free >= share {
            self.free_bandwidth = DataRate::b(free - share);
            let id = QueueId::from(self.queues.len() as u32);
            if self
                .queues
                .push(QueueStatus::new(id, share, self.port_bandwidth.as_u64()))
                .is_err()
            {
                return None;
            }
            return Some(id);
        }
        None
    }
    fn request_transmission(&mut self, transmission: &Transmission) -> Result<(), Error> {
        let q_id = transmission.queue;
        let q = self
            .queues
            .get_mut(q_id.0 as usize)
            .ok_or(Error::NoSuchQueue(q_id))?;
        _ = q.submit(transmission.encoded);
        Ok(())
    }

    fn record_transmission(&mut self, transmission: &Transmission) -> Result<(), Error> {
        let mut consumed = false;
        for q in self.queues.iter_mut() {
            if q.id == transmission.queue {
                q.transmit = false;
                _ = q.consume(&transmission.encoded)?;
                consumed = true;
            } else {
                let duration_blocked = Duration::from_micros(
                    (transmission.encoded * 1_000_000) / self.port_bandwidth.as_u64(),
                );
                _ = q.restore(&duration_blocked)?;
            }
        }
        if consumed {
            Ok(())
        } else {
            Err(Error::InvalidTransmission(*transmission))
        }
    }

    /// Determines which queue is next.
    /// - queues: The available queues sorted by their priority.
    /// It is assumed that each queue services frames of a limited size so there is a lo_credit for each queue.
    fn next_queue(&mut self) -> Option<QueueId> {
        for q in self.queues.iter_mut() {
            trace!(
                "Queue {} has backlog {} and credit {}",
                q.id,
                q.backlog,
                q.credit
            );
            if q.transmit_allowed() && q.backlog > 0 {
                q.transmit = true;
                return Some(q.id);
            }
        }
        None
    }

    fn restore_all(&mut self, restore: Duration) -> Result<(), Error> {
        for q in self.queues.iter_mut() {
            if q.backlog > 0 {
                _ = q.restore(&restore)?;
            }
        }
        Ok(())
    }

    fn get_backlog(&self, queue: &QueueId) -> Option<u64> {
        self.queues.get(queue.0 as usize).map(|q| q.backlog)
    }
}

/// A stream that is managed by the shaper.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
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
    fn consume(&mut self, encoded: &u64) -> Result<u64, Error> {
        if !self.transmit_allowed() {
            return Err(Error::TransmitNotAllowed);
        }
        let consumed = encoded;

        self.credit -= *consumed as i128;
        if self.backlog >= *consumed {
            self.backlog -= consumed;
        } else {
            self.backlog = 0;
        }
        trace!("Consumed {consumed:?} and removed {consumed:?} from backlog");
        Ok(*consumed)
    }

    /// Increases the credit, while the queue was blocked by the port transmitting conflicting
    /// traffic or a higher priority queue has queued frames.
    fn restore(&mut self, time: &Duration) -> Result<u64, Error> {
        // Another queue is transmitting and this queue is blocked
        //, otherwise this queue would have been selected.
        // Or the queue has nothing to transmit, but the credit is still negative.
        if !self.transmit && (self.backlog > 0 || self.credit < 0) {
            match time.checked_mul(self.idle_slope as u32) {
                Some(credited_bytes) => {
                    let credited_bytes = credited_bytes.as_secs() as i128;
                    self.credit += credited_bytes;
                    Ok(credited_bytes as u64)
                }
                None => {
                    error!("Failed to restore credit to queue, because a calculation step failed");
                    Err(Error::Unknown) // TODO better error handling
                }
            }
        } else {
            if self.backlog == 0 && !self.transmit {
                self.credit = 0;
            }
            Ok(0)
        }
    }

    fn submit(&mut self, bits: u64) -> Result<u64, u64> {
        self.backlog = self.backlog.checked_add(bits).ok_or(bits)?;
        trace!("Submitted {bits} to queue {}", self.id);
        Ok(self.backlog)
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

        assert!(!queue.transmit_allowed());
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
        assert!(queue.transmit_allowed());
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
        let mut s = CreditBasedShaper::<2>::new(DataRate::b(128_000));
        _ = s.add_queue(DataRate::b(90_000)).unwrap();
        let id = s.add_queue(DataRate::b(90_000));
        assert!(id.is_none())
    }

    #[test]
    fn given_enough_remaining_bandwidth_when_queue_added_then_ok() {
        let mut s = CreditBasedShaper::<2>::new(DataRate::b(128_000));
        _ = s.add_queue(DataRate::b(64_000)).unwrap();
        let id = s.add_queue(DataRate::b(50_000));
        assert!(id.is_some())
    }

    #[test]
    fn given_exactly_enough_remaining_bandwidth_when_queue_added_then_ok() {
        let mut s = CreditBasedShaper::<2>::new(DataRate::b(128_000));
        _ = s.add_queue(DataRate::b(64_000)).unwrap();
        let id = s.add_queue(DataRate::b(64_000));
        assert!(id.is_some())
    }

    #[test]
    fn given_two_queues_high_credit_and_backlog_when_both_transmit_then_bandwidth_usage_below_limit(
    ) {
        let mut s = CreditBasedShaper::<2>::new(DataRate::b(100_000));
        let q0_id = s.add_queue(DataRate::b(60_000)).unwrap();
        let q1_id = s.add_queue(DataRate::b(40_000)).unwrap();

        const MTU_Q1: u32 = 1_000;
        const MTU_Q2: u32 = 4_000;

        let transmissions = [
            Transmission::new(q0_id, 2000),
            Transmission::new(q1_id, 2000),
            Transmission::new(q1_id, 2000),
            Transmission::new(q0_id, 2000),
            Transmission::new(q0_id, 2000),
            Transmission::new(q1_id, 2000),
            Transmission::new(q0_id, 2000),
            Transmission::new(q1_id, 2000),
            Transmission::new(q1_id, 2000),
            Transmission::new(q0_id, 2000),
            Transmission::new(q0_id, 2000),
            Transmission::new(q1_id, 2000),
        ];

        for t in transmissions.iter() {
            s.request_transmission(t).unwrap();
        }

        let mut total_byte: u64 = 0;
        let mut total_trans: u64 = 0;

        // TODO should probably be an iterator
        while let Some(next_q) = s.next_queue() {
            // ... transmit
            if next_q == QueueId(0) {
                let t = Transmission::new(next_q, 2000);
                s.record_transmission(&t).unwrap();
                total_byte += MTU_Q1 as u64;
                total_trans += 2000;
            } else {
                let t = Transmission::new(next_q, 5000);
                s.record_transmission(&t).unwrap();
                total_byte += MTU_Q2 as u64;
                total_trans += 5000;
            }
            // TODO assert that queues do not accumulate more than their maximum credit
            //assert!();
            assert!(true)
        }
    }

    #[test]
    fn given_empty_queue_with_credit_when_next_queue_then_none() {
        let mut shaper = CreditBasedShaper::<1>::new(DataRate::b(10_000_000));
        _ = shaper.add_queue(DataRate::b(1_000_000)).unwrap();
        let mut status = shaper.queues.get_mut(0).unwrap();
        status.backlog = 0;
        assert!(shaper.next_queue().is_none());
    }
}

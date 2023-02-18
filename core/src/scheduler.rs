use crate::prelude::VirtualLinkId;
use core::{fmt::Debug, time::Duration};
use heapless::Vec;
use log::{info, trace};

/// A scheduler for virtual links.
pub trait Scheduler: Debug {
    /// Gets the next virtual link that is allowed to transmit one message to the network.
    // Returns None if no link is allowed to transmit a frame at the moment.
    fn next(&mut self, current_time: Duration) -> Option<VirtualLinkId>;
}

/// The deadline of a window in which a virtual link is to be scheduled next.
#[derive(Debug, Copy, Clone)]
struct Window {
    /// Virtual link this window belongs to.
    vl: VirtualLinkId,
    /// Period to schedule the window at.
    /// The next period is measured from the last time the start of the last time the partition has been scheduled or from the beginning of time.
    period: Duration,
    /// The next instant at which this window should be scheduled.
    next: Duration,
}

impl Window {
    fn is_due(&self, current_time: Duration) -> bool {
        self.next <= current_time
    }
}

/// A scheduler that uses simple round-robin scheduling together with dealines for every virtual link.
/// A virtual link may be scheduled at multiple intervals, although this may not make much sense, depending on which requirements on jitter this has.
#[derive(Debug, Clone)]
pub struct DeadlineRrScheduler<const SLOTS: usize> {
    /// The next window in the round-robin schedule.
    last_window: usize,
    /// The windows inside of the round-robin schedule.
    windows: Vec<Window, SLOTS>,
}

impl<const SLOTS: usize> DeadlineRrScheduler<SLOTS> {
    /// Creates a new scheduler.
    pub fn new() -> Self {
        Self {
            last_window: usize::default(),
            windows: Vec::default(),
        }
    }

    /// Add a new window.
    pub fn insert(&mut self, vl: VirtualLinkId, period: Duration) -> Result<(), ()> {
        self.windows
            .push(Window {
                vl,
                period,
                next: period,
            })
            .or(Err(()))
    }

    /// Clears the schedule.
    pub fn clear(&mut self) {
        self.windows.clear();
    }
}

impl<const SLOTS: usize> Scheduler for DeadlineRrScheduler<SLOTS> {
    fn next(&mut self, current_time: Duration) -> Option<VirtualLinkId> {
        // Try all windows of one round-robin and return None if none of them are past their deadline.
        for i in 1..=SLOTS {
            let next_window = (self.last_window + i) % SLOTS;
            let window = self.windows[next_window];
            if window.is_due(current_time) {
                self.last_window = next_window;
                self.windows[next_window].next = current_time
                    .checked_add(window.period)
                    .unwrap_or_else(|| current_time);

                // Return the next window
                trace!("Scheduled VL {}", window.vl);
                return Some(window.vl);
            }
        }
        None
    }
}

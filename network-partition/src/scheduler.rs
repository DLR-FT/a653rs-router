use crate::prelude::*;
use core::{fmt::Debug, time::Duration};
use heapless::Vec;
use log::{trace, warn};

/// Gets the next virtual link that is scheduled.
/// To be implemented by the concrete scheduler implementation.
pub trait IoScheduler: Debug {
    /// Get the next scheduled virtual link, if one is to be scheduled at the current time.
    fn schedule_next(&mut self, current_time: &Duration) -> Option<VirtualLinkId>;
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
    fn is_due(&self, current_time: &Duration) -> bool {
        self.next <= *current_time
    }
}

/// A scheduler that uses simple round-robin scheduling together with dealines for every virtual link.
/// A virtual link may be scheduled at multiple intervals, although this may not make much sense, depending on which requirements on jitter this has.
// A schedule of the deadline-based round-robin scheduler.
#[derive(Debug, Clone)]
pub struct DeadlineRrScheduler<const SLOTS: usize> {
    /// The next window in the round-robin schedule.
    last_window: usize,
    /// The windows inside of the round-robin schedule.
    windows: Vec<Window, SLOTS>,
}

impl<const SLOTS: usize> DeadlineRrScheduler<SLOTS> {
    /// Creates a new scheduler.
    pub fn new(cfg: &DeadlineRrScheduleConfig<SLOTS>) -> Self {
        Self {
            last_window: usize::default(),
            windows: cfg
                .slots
                .iter()
                .map(|s| Window {
                    vl: s.vl,
                    period: s.period,
                    next: s.period,
                })
                .collect(),
        }
    }
}

impl<const SLOTS: usize> IoScheduler for DeadlineRrScheduler<SLOTS> {
    fn schedule_next(&mut self, current_time: &Duration) -> Option<VirtualLinkId> {
        // Try all windows of one round-robin and return None if none of them are past their deadline.
        for i in 1..=SLOTS {
            let next_window = (self.last_window + i) % self.windows.len();
            let window = self.windows[next_window];
            if window.is_due(current_time) {
                // Check if clock skipped for some reason.
                if let Some(t) = current_time.checked_sub(Duration::from_secs(10)) {
                    if t > window.next {
                        warn!("The system clock is {current_time:?} and this does not seem right. Ignoring this value.");
                        return None;
                    }
                }
                self.last_window = next_window;
                let next = current_time
                    .checked_add(window.period)
                    .unwrap_or(*current_time);
                self.windows[next_window].next = next;

                trace!("Scheduled VL {}, next window at {:?}", window.vl, next);

                // Return the next window
                trace!("Scheduled VL {}", window.vl);
                return Some(window.vl);
            }
        }
        None
    }
}

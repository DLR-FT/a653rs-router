use crate::{reconfigure::CfgError, types::VirtualLinkId};

use a653rs::prelude::{ApexTimeP4Ext, SystemTime};
use core::{
    fmt::{Debug, Display, Formatter},
    time::Duration,
};
use heapless::Vec;
use log::trace;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Gets the next virtual link that is scheduled.
/// To be implemented by the concrete scheduler implementation.
pub trait Scheduler {
    /// Get the next scheduled virtual link, if one is to be scheduled at the
    /// current time.
    fn schedule_next(&mut self, current_time: &Duration) -> Option<VirtualLinkId>;

    /// Reconfigures the scheduler.
    fn reconfigure(&mut self, vls: &[(VirtualLinkId, Duration)]) -> Result<(), CfgError>;
}

/// The deadline of a window in which a virtual link is to be scheduled next.
#[derive(Debug, Copy, Clone)]
struct Window {
    /// Virtual link this window belongs to.
    vl: VirtualLinkId,
    /// Period to schedule the window at.
    /// The next period is measured from the last time the start of the last
    /// time the partition has been scheduled or from the beginning of time.
    period: Duration,
    /// The next instant at which this window should be scheduled.
    next: Duration,
}

impl Window {
    fn is_due(&self, current_time: &Duration) -> bool {
        self.next <= *current_time
    }
}

/// A scheduler that uses simple round-robin scheduling together with dealines
/// for every virtual link. A virtual link may be scheduled at multiple
/// intervals, although this may not make much sense, depending on which
/// requirements on jitter this has.
// A schedule of the deadline-based round-robin scheduler.
#[derive(Default, Debug, Clone)]
pub struct DeadlineRrScheduler<const SLOTS: usize> {
    /// The next window in the round-robin schedule.
    last_window: usize,
    /// The windows inside of the round-robin schedule.
    windows: Vec<Window, SLOTS>,
}

impl<const SLOTS: usize> DeadlineRrScheduler<SLOTS> {
    /// Constructs a new DeadlineRrScheduler.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<const SLOTS: usize> Scheduler for DeadlineRrScheduler<SLOTS> {
    fn schedule_next(&mut self, current_time: &Duration) -> Option<VirtualLinkId> {
        if self.windows.is_empty() {
            return None;
        }

        // Try all windows of one round-robin and return None if none of them are past
        // their deadline.
        for i in 1..=SLOTS {
            let next_window = (self.last_window + i) % self.windows.len();
            let window = self.windows[next_window];
            if window.is_due(current_time) {
                // Check if clock skipped for some reason.
                if let Some(t) = current_time.checked_sub(Duration::from_secs(15)) {
                    if t > window.next {
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

    fn reconfigure(&mut self, vls: &[(VirtualLinkId, Duration)]) -> Result<(), CfgError> {
        self.last_window = 0;
        self.windows = vls
            .iter()
            .map(|(vl, period)| Window {
                vl: *vl,
                period: *period,
                next: *period,
            })
            .collect();
        Ok(())
    }
}

/// A slot inside the round-robin scheduler.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct DeadlineRrSlot {
    /// Virtual link to schedule in this slot.
    pub vl: VirtualLinkId,

    /// Periodic after which to schedule this slot again after the last time it
    /// has been scheduled.
    pub period: Duration,
}

/// Source for the system time.
pub trait TimeSource {
    /// Gets the current system time.
    fn get_time(&self) -> Result<Duration, InvalidTimeError>;
}

impl<T: ApexTimeP4Ext> TimeSource for T {
    fn get_time(&self) -> Result<Duration, InvalidTimeError> {
        match <T as ApexTimeP4Ext>::get_time() {
            SystemTime::Normal(d) => Ok(d),
            SystemTime::Infinite => Err(InvalidTimeError {}),
        }
    }
}

/// The time returned by the system was invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidTimeError;

/// An error occured when scheduling a virtual link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleError {
    kind: ScheduleErrorKind,
}

/// Schedule error type.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ScheduleErrorKind {
    /// The system time was not normal.
    SystemTime(InvalidTimeError),
}

impl From<InvalidTimeError> for ScheduleError {
    fn from(value: InvalidTimeError) -> Self {
        ScheduleError {
            kind: ScheduleErrorKind::SystemTime(value),
        }
    }
}
impl Display for ScheduleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match &self.kind {
            ScheduleErrorKind::SystemTime(e) => write!(f, "The system time was invalid: {e:?}"),
        }
    }
}

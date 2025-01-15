use a653rs::prelude::SystemTime;
use a653rs::prelude::{Error as ApexError, Name, Process, StartContext};
use a653rs::{bindings::*, prelude::ProcessAttribute};
use core::fmt::Debug;
use core::time::Duration;

/// Router process
#[derive(Debug)]
pub struct RouterProcess<H: ApexProcessP4> {
    inner: Process<H>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(pub ApexError);

impl From<ApexError> for ProcessError {
    fn from(value: ApexError) -> Self {
        Self(value)
    }
}

impl<H: ApexProcessP4> RouterProcess<H> {
    /// Runs the router process.
    ///
    /// # Errors
    ///
    /// This function will return an error if creating the process or starting
    /// the process fails.
    pub fn create(
        ctx: &mut StartContext<H>,
        name: Name,
        period: Duration,
        time_capacity: Duration,
        stack_size: StackSize,
        entry_point: extern "C" fn(),
    ) -> Result<Self, ProcessError> {
        Ok(Self {
            inner: ctx.create_process(ProcessAttribute {
                period: SystemTime::Normal(period),
                time_capacity: SystemTime::Normal(time_capacity),
                entry_point,
                stack_size,
                base_priority: 1,
                deadline: Deadline::Soft,
                name,
            })?,
        })
    }

    /// Starts the router process.
    ///
    /// # Errors
    /// Returns an error wrapping the APEX error if starting the process fails
    /// for any reason.
    pub fn start(&self) -> Result<(), ProcessError> {
        self.inner.start().map_err(ProcessError::from)
    }
}

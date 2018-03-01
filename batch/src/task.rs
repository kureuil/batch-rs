//! A trait representing an executable task.

use std::str::FromStr;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use error::{Error, ErrorKind, Result};

/// An executable task and its related metadata (name, queue, timeout, etc.)
///
/// In most cases, you should be deriving this trait instead of implementing it manually yourself.
///
/// # Examples
///
/// Using the provided defaults:
///
/// ```rust
/// #[macro_use]
/// extern crate batch;
/// extern crate serde;
/// #[macro_use]
/// extern crate serde_derive;
///
/// #[derive(Deserialize, Serialize, Task)]
/// #[task_routing_key = "emails"]
/// struct SendConfirmationEmail;
///
/// #
/// # fn main() {}
/// ```
///
/// Overriding the provided defaults:
///
/// ```rust
/// #[macro_use]
/// extern crate batch;
/// extern crate serde;
/// #[macro_use]
/// extern crate serde_derive;
///
/// struct App;
///
/// #[derive(Deserialize, Serialize, Task)]
/// #[task_name = "batch-rs:send-password-reset-email"]
/// #[task_routing_key = "emails"]
/// #[task_timeout = "120"]
/// #[task_retries = "0"]
/// struct SendPasswordResetEmail;
///
/// #
/// # fn main() {}
/// ```
pub trait Task: DeserializeOwned + Serialize {
    /// A should-be-unique human-readable ID for this task.
    fn name() -> &'static str;

    /// The exchange the task will be published to.
    fn exchange() -> &'static str;

    /// The routing key associated to this task.
    fn routing_key() -> &'static str;

    /// The number of times this task must be retried in case of error.
    fn retries() -> u32;

    /// An optional duration representing the time allowed for this task's handler to complete.
    fn timeout() -> Option<Duration>;

    /// The priority associated to this task.
    fn priority() -> Priority;
}

/// The different priorities that can be assigned to a `Task`.
///
/// The default value is `Priority::Normal`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// The lowest available priority for a task.
    Trivial,
    /// A lower priority than `Priority::Normal` but higher than `Priority::Trivial`.
    Low,
    /// The default priority for a task.
    Normal,
    /// A higher priority than `Priority::Normal` but higher than `Priority::Critical`.
    High,
    /// The highest available priority for a task.
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

impl FromStr for Priority {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "trivial" => Ok(Priority::Trivial),
            "low" => Ok(Priority::Low),
            "normal" => Ok(Priority::Normal),
            "high" => Ok(Priority::High),
            "critical" => Ok(Priority::Critical),
            _ => Err(ErrorKind::InvalidPriority)?,
        }
    }
}

impl Priority {
    /// Return the priority as a `u8` ranging from 0 to 4.
    ///
    /// # Examples
    ///
    /// ```
    /// use batch::Priority;
    ///
    /// let p = Priority::Normal;
    /// assert_eq!(p.to_u8(), 2);
    /// ```
    pub fn to_u8(&self) -> u8 {
        match *self {
            Priority::Trivial => 0,
            Priority::Low => 1,
            Priority::Normal => 2,
            Priority::High => 3,
            Priority::Critical => 4,
        }
    }
}

/// The `Perform` trait allow marking a `Task` as executable.
///
/// # Example
///
/// ```
/// #[macro_use]
/// extern crate batch;
/// extern crate serde;
/// #[macro_use]
/// extern crate serde_derive;
///
/// use batch::Perform;
///
/// #[derive(Serialize, Deserialize, Task)]
/// #[task_routing_key = "emails"]
/// struct SendPasswordResetEmail;
///
/// impl Perform for SendPasswordResetEmail {
///     type Context = ();
///
///     fn perform(&self, _ctx: Self::Context) {
///         println!("Sending password reset email...");
///     }
/// }
///
/// # fn main() {}
/// ```
pub trait Perform {
    /// The type of the context value that will be given to this task's handler.
    type Context;

    /// Perform the task's duty.
    fn perform(&self, Self::Context);
}

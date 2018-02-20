//! A trait representing an executable task.

use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

use error::Result;

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
/// #[macro_use]
/// extern crate serde;
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
/// #[macro_use]
/// extern crate serde;
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
}

/// The `Perform` trait allow marking a `Task` as executable.
///
/// # Example
///
/// ```
/// #[macro_use]
/// extern crate batch;
/// #[macro_use]
/// extern crate serde;
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

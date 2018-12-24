//! A trait representing a job.

use std::fmt;
use std::time::Duration;

use failure::Error;
use futures::Future;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Factory;

/// A job and its related metadata (name, queue, timeout, etc.)
///
/// In most cases, you should be using the `job` proc-macro instead of implementing it manually yourself.
///
/// # Examples
///
/// ```rust
/// # extern crate batch;
/// use batch::job;
///
/// #[job(name = "batch-example.send-confirmation-email")]
/// fn send_confirmation_email(email: String) {
/// # drop(email);
///     // ...
/// }
/// #
/// # fn main() {}
/// ```
pub trait Job: Serialize + for<'de> Deserialize<'de> {
    /// A should-be-unique human-readable ID for this job.
    const NAME: &'static str;

    /// The return type of the `perform` method.
    type PerformFuture: Future<Item = (), Error = Error> + Send + 'static;

    /// Perform the background job.
    fn perform(self, context: &Factory) -> Self::PerformFuture;

    /// The number of times this job must be retried in case of error.
    ///
    /// This function is meant to be overriden by the user to return a value different from the associated
    /// constant depending on the contents of the actual job.
    ///
    /// By default, a job will be retried 25 times.
    fn retries(&self) -> u32 {
        25
    }

    /// An optional duration representing the time allowed for this job's handler to complete.
    ///
    /// This function is meant to be overriden by the user to return a value different from the associated
    /// constant depending on the contents of the actual job.
    ///
    /// By default, a job is allowed to run 30 minutes.
    fn timeout(&self) -> Duration {
        Duration::from_secs(30 * 60)
    }
}

/// Various metadata for a job.
#[derive(Clone, Serialize, Deserialize)]
pub struct Properties {
    /// The language in which the job was created.
    pub lang: String,
    /// The name of the job.
    pub task: String,
    /// The ID of this job.
    ///
    /// To guarantee the uniqueness of this ID, a UUID version 4 used.
    pub id: Uuid,
    /// The ID of the greatest ancestor of this job, if there is one.
    pub root_id: Option<Uuid>,
    /// The ID of the direct ancestor of this job, if there is one.
    pub parent_id: Option<Uuid>,
    /// The ID of the group this job is part of, if there is one.
    pub group: Option<Uuid>,
    /// Timelimits for this job.
    ///
    /// The first duration represents the soft timelimit while the second duration represents the hard timelimit.
    pub timelimit: (Option<Duration>, Option<Duration>),
    /// The content type of the job once serialized.
    pub content_type: String,
    /// The content encoding of the job once serialized.
    pub content_encoding: String,
    __non_exhaustive: (),
}

impl Properties {
    /// Create a new `Properties` instance from a task name.
    pub fn new<T: ToString>(task: T) -> Self {
        Properties {
            lang: "rs".to_string(),
            task: task.to_string(),
            id: Uuid::new_v4(),
            root_id: None,
            parent_id: None,
            group: None,
            timelimit: (None, None),
            content_type: "application/json".to_string(),
            content_encoding: "utf-8".to_string(),
            __non_exhaustive: (),
        }
    }
}

impl fmt::Debug for Properties {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Properties")
            .field("content_encoding", &self.content_encoding)
            .field("content_type", &self.content_type)
            .field("lang", &self.lang)
            .field("task", &self.task)
            .field("id", &self.id)
            .field("timelimit", &self.timelimit)
            .field("root_id", &self.root_id)
            .field("parent_id", &self.parent_id)
            .field("group", &self.group)
            .finish()
    }
}

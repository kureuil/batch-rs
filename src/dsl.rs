//! A fluent interface to configure & send jobs.

use failure::Error;
use futures::Future;

use job::Job;

/// A query contructor.
pub trait With<J: Job> {
    /// Concrete type of the query.
    type Query: Deliver;

    /// Create a new query.
    fn with(&self, job: J) -> Self::Query;
}

/// A deliverable job.
pub trait Deliver {
    /// The return type of the `deliver` method.
    type DeliverFuture: Future<Item = (), Error = Error> + Send;

    /// Deliver the job. Consumes the query.
    fn deliver(self) -> Self::DeliverFuture;
}

use failure::Error;
use futures::Future;

use job::Properties;

/// A message that can be received from a broker.
pub trait Delivery: Send {
    /// The return type of the `ack` method.
    type AckFuture: Future<Item = (), Error = Error> + Send;

    /// The return type of the `reject` method.
    type RejectFuture: Future<Item = (), Error = Error> + Send;

    /// Get a reference to the serialized `Job` instance associated to this delivery.
    fn payload(&self) -> &[u8];

    /// Get a reference to the `Properties` instance associated to this delivery.
    fn properties(&self) -> &Properties;

    /// Ack the delivery.
    ///
    /// Acking a delivery marks its associated job as completed.
    fn ack(self) -> Self::AckFuture;

    /// Reject the delivery.
    ///
    /// Rejecting a delivery means its associated job hasn't completed successfully and should be retried (if possible).
    /// A job cannot be retried if it has reached its retries limit.
    fn reject(self) -> Self::RejectFuture;
}

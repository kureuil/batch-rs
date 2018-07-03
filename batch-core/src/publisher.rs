use failure::Error;
use futures::Sink;

use dispatch::Dispatch;

/// A publisher of dispatches to be sent to a message broker.
pub trait Publisher:
    Sink<SinkItem = <Self as Publisher>::Dispatch, SinkError = Error> + Send + Clone
{
    /// The type of dispatches published to the broker.
    type Dispatch: Dispatch;
}

/// Publish a dispatch to a broker.
pub trait ToPublisher {
    /// The return type of the `publisher` method.
    type Publisher: Publisher;

    /// Publish the given message to a broker.
    fn to_publisher(&mut self) -> Self::Publisher;
}

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

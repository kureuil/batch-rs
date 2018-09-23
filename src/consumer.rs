use failure::Error;
use futures::{Future, Stream};

use delivery::Delivery;

/// A consumer of deliveries coming from a message broker.
pub trait Consumer: Stream<Item = <Self as Consumer>::Delivery, Error = Error> + Send {
    /// The type of message consumed.
    type Delivery: Delivery;
}

/// Consume messages from a broker.
pub trait ToConsumer {
    /// The type of `Consumer` returned by `to_consumer`.
    type Consumer: Consumer;

    /// The return type of the `to_consumer` method.
    type ToConsumerFuture: Future<Item = Self::Consumer, Error = Error> + Send;

    /// Create a consumer of deliveries coming from the given sources.
    fn to_consumer(
        &mut self,
        sources: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self::ToConsumerFuture;
}

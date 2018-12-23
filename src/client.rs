use failure::Error;
use futures::{Future, Stream};

use crate::{Delivery, Dispatch};

/// A message broker client.
///
/// A `Clone` implementation is required by the `Client` trait and, due to the
/// current implementation of async I/O in Rust, should be as cheap as possible
/// (first-party implementations commonly use an `Arc` to fulfill this
/// requirement). As soon as async/await will be available in the language, this
/// performance constraint could be relaxed.
pub trait Client: Clone {
    /// The type of the future returned by `send`.
    type SendFuture: Future<Item = (), Error = Error> + Send;

    /// Send a dispatch to the message broker.
    fn send(&mut self, dispatch: Dispatch) -> Self::SendFuture;

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

/// A consumer of deliveries coming from a message broker.
pub trait Consumer: Stream<Item = <Self as Consumer>::Delivery, Error = Error> + Send {
    /// The type of message yielded by the consumer.
    type Delivery: Delivery;
}

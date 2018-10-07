use failure::Error;
use futures::Future;

use Dispatch;

/// A message broker client.
///
/// A `Clone` implementation is required by the `Client` trait and, due to the
/// current implementation of async I/O in Rust, should be as cheap as possible
/// (first-party implementations commonly use an `Arc`). As soon as async/await
/// will be available in the language, this performance constraint could be
/// relaxed.
pub trait Client: Clone {
    /// The type of the future returned by `send`.
    type SendFuture: Future<Item = (), Error = Error>;

    /// Send a dispatch to the message broker.
    fn send(&mut self, dispatch: Dispatch) -> Self::SendFuture;
}

//! Utilities for declaring resources

use failure::Error;
use futures::Future;

use crate::Factory;

/// A resource that has to be declared to be used.
///
/// This trait is meant to be implemented by adapters to message brokers.
pub trait Queue: Sized {
    /// The key used when publishing a job to this queue.
    const SOURCE: &'static str;

    /// The key used when consuming jobs from this queue.
    const DESTINATION: &'static str;

    /// The return type of the `callbacks` method.
    type CallbacksIterator: Iterator<
        Item = (
            &'static str,
            fn(&[u8], &Factory) -> Box<dyn Future<Item = (), Error = Error> + Send>,
        ),
    >;

    /// Get the callbacks associated to this queue.
    ///
    /// A callback is represented by a `(&str, Fn(&[u8], Factory) -> Future)` tuple. The first element is
    /// the key associated to the callback, typically this is the name of the job. The second element is the proper
    /// callback function. A callback function takes two parameters:
    /// * `payload`: A slice of bytes, it is the serialized representation of the to be handled job.
    /// * `container`: A type-map containing the types provided by the executor (usually some kind `Worker`).
    fn callbacks() -> Self::CallbacksIterator;
}

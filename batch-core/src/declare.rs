//! Utilities for declaring resources

use failure::Error;
use futures::Future;

use Factory;

/// A resource that has to be declared to be used.
///
/// This trait is meant to be implemented by adapters to message brokers. For example, both `batch_rabbitmq::Exchange`
/// and `batch_rabbitmq::Queue` implement it.
pub trait Declare: Sized {
    /// The name of the declared resource.
    const NAME: &'static str;

    /// Data used during the declaration.
    type Input;

    /// The type that will be declared.
    type Output;

    /// The return type of the method.
    type DeclareFuture: Future<Item = Self, Error = Error> + Send;

    /// Declare the current resource and create an instance of it.
    fn declare(
        declarator: &mut (impl Declarator<Self::Input, Self::Output> + Send + 'static),
    ) -> Self::DeclareFuture;
}

/// A trait for declaring resources.
///
/// This trait is meant to be implemented by adapters to message brokers.
pub trait Declarator<I, O> {
    /// The return type of the method.
    type DeclareFuture: Future<Item = O, Error = Error> + Send;

    /// Declare the given resource.
    fn declare(&mut self, resource: I) -> Self::DeclareFuture;
}

/// Get the callbacks associated to a resource.
///
/// This is typically used to return job handlers associated to queues.
pub trait Callbacks {
    /// The return type of the `callbacks` method.
    type Iterator: Iterator<
        Item = (
            String,
            fn(&[u8], Factory) -> Box<Future<Item = (), Error = Error> + Send>,
        ),
    >;

    /// Get a list of callbacks.
    ///
    /// A callback is represented by a `(String, Box<CallbackFn<Self::Delivery> + Send>)` tuple. The first element is
    /// the key associated to the callback, typically this is the name of the job. The second element is the proper
    /// callback function. A callback function takes two parameters:
    /// * `payload`: A slice of bytes, it is the serialized representation of the to be handled job.
    /// * `container`: A type-map containing the types provided by the executor (usually some kind `Worker`).
    fn callbacks(&self) -> Self::Iterator;
}

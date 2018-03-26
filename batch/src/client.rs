//! Batch client.

use std::iter::FromIterator;

use futures::{future, Future};
use lapin::channel::{BasicProperties, BasicPublishOptions};
use tokio_core::reactor::Handle;

use error::{Error, ErrorKind};
use rabbitmq::{Exchange, ExchangeBuilder, Publisher};

/// A builder to ease the construction of `Client` instances.
#[derive(Debug)]
pub struct ClientBuilder {
    connection_url: String,
    exchanges: Vec<Exchange>,
    handle: Option<Handle>,
}

impl Default for ClientBuilder {
    fn default() -> ClientBuilder {
        ClientBuilder {
            connection_url: "amqp://localhost/%2f".into(),
            exchanges: Vec::new(),
            handle: None,
        }
    }
}

impl ClientBuilder {
    /// Create a new `ClientBuilder` instance.
    pub fn new() -> Self {
        ClientBuilder::default()
    }

    /// Set the URL used to connect to `RabbitMQ`.
    ///
    /// The URL must be a valid AMQP connection URL (ex: `amqp://localhost/%2f`) using either the
    /// `amqp` protocol or the `amqps` protocol.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ClientBuilder;
    ///
    /// let builder = ClientBuilder::new()
    ///     .connection_url("amqp://guest:guest@localhost:5672/%2f");
    /// ```
    pub fn connection_url(mut self, url: &str) -> Self {
        self.connection_url = url.into();
        self
    }

    /// Add exchanges to be declared when connecting to `RabbitMQ`.
    ///
    /// See `exchange` documentation.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::{exchange, ClientBuilder};
    ///
    /// let exchanges = vec![
    ///     exchange("batch.example"),
    /// ];
    /// let builder = ClientBuilder::new()
    ///     .exchanges(exchanges);
    /// ```
    pub fn exchanges<EIter>(mut self, exchanges: EIter) -> Self
    where
        EIter: IntoIterator<Item = ExchangeBuilder>,
    {
        self.exchanges
            .extend(exchanges.into_iter().map(|e| e.build()));
        self
    }

    /// Set the `Handle` to the Tokio reactor that should be used by the `Worker`.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate batch;
    /// # extern crate failure;
    /// # extern crate tokio_core;
    /// #
    /// use batch::ClientBuilder;
    /// # use failure::Error;
    /// use tokio_core::reactor::Core;
    ///
    /// # fn main() {
    /// #     example().unwrap();
    /// # }
    /// #
    /// # fn example() -> Result<(), Error> {
    /// let core = Core::new()?;
    /// let handle = core.handle();
    /// let builder = ClientBuilder::new()
    ///     .handle(handle);
    /// # Ok(())
    /// # }
    /// ```
    pub fn handle(mut self, handle: Handle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Build a new `Client` instance from this builder data.
    pub fn build(self) -> Box<Future<Item = Client, Error = Error>> {
        if self.handle.is_none() {
            return Box::new(future::err(ErrorKind::NoHandle.into()));
        }
        let task = Publisher::new_with_handle(
            &self.connection_url,
            self.exchanges,
            self.handle.unwrap(),
        ).and_then(|publisher| Ok(Client { publisher }));
        Box::new(task)
    }
}

/// The `Client` is responsible for sending tasks to the broker.
#[derive(Clone, Debug)]
pub struct Client {
    publisher: Publisher,
}

impl Client {
    /// Send a job to the client's message broker.
    ///
    /// Once a job is sent to the message broker, it is transmitted to a Worker currently
    /// receiving jobs from the same broker.
    pub(crate) fn send(
        &self,
        exchange: &str,
        routing_key: &str,
        job: &[u8],
        options: &BasicPublishOptions,
        properties: BasicProperties,
    ) -> Box<Future<Item = (), Error = Error>> {
        let task = self.publisher
            .send(exchange, routing_key, job, options, properties);
        Box::new(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}

    fn assert_sync<T: Sync>() {}

    #[test]
    fn test_auto_impl_traits() {
        assert_send::<Client>();
        assert_sync::<Client>();
    }
}

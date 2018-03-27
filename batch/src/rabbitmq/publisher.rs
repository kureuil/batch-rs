use std::fmt;
use std::result::Result as StdResult;

use futures::{future, Future};
use lapin::channel::{BasicProperties, BasicPublishOptions, Channel};
use lapin::client::Client;
use tokio_core::reactor::Handle;

use error::{Error, ErrorKind};
use rabbitmq::common::{connect, declare_exchanges};
use rabbitmq::stream::Stream;
use rabbitmq::types::Exchange;

/// An AMQP based publisher for the Batch distributed task queue.
#[derive(Clone)]
pub struct Publisher {
    channel: Channel<Stream>,
    client: Client<Stream>,
}

impl fmt::Debug for Publisher {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(f, "Publisher {{ }}")
    }
}

impl Publisher {
    /// Create a `Publisher` instance from a RabbitMQ URI and an explicit tokio handle.
    pub fn new_with_handle<E>(
        connection_url: &str,
        exchanges_iter: E,
        handle: Handle,
    ) -> Box<Future<Item = Self, Error = Error>>
    where
        E: IntoIterator<Item = Exchange>,
    {
        let exchanges = exchanges_iter.into_iter().collect::<Vec<_>>();

        let task = connect(connection_url, handle)
            .and_then(|client| {
                client
                    .create_channel()
                    .map_err(|e| ErrorKind::Rabbitmq(e).into())
                    .join(future::ok(client))
            })
            .and_then(move |(channel, client)| {
                let channel_ = channel.clone();
                declare_exchanges(exchanges, channel_)
                    .map_err(|e| ErrorKind::Rabbitmq(e).into())
                    .map(|_| (channel, client))
            })
            .map(move |(channel, client)| Publisher { client, channel });
        Box::new(task)
    }

    /// Send a job to the broker.
    ///
    /// Returns a `Future` that completes once the job is sent to the broker.
    pub fn send(
        &self,
        exchange: &str,
        routing_key: &str,
        serialized: &[u8],
        options: &BasicPublishOptions,
        properties: BasicProperties,
    ) -> Box<Future<Item = (), Error = Error>> {
        let task = self.channel
            .basic_publish(exchange, routing_key, serialized, options, properties)
            .and_then(move |_| future::ok(()))
            .map_err(|e| ErrorKind::Rabbitmq(e).into());
        Box::new(task)
    }
}

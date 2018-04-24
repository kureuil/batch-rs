use std::fmt;
use std::result::Result as StdResult;
use std::sync::Arc;

use futures::{future, Future};
use lapin::channel::{BasicProperties, BasicPublishOptions, Channel};
use lapin::client::Client;
use tokio_reactor::Handle;

use error::{Error, ErrorKind};
use rabbitmq::common::{connect, declare_exchanges, declare_queues, HeartbeatHandle};
use rabbitmq::stream::Stream;
use rabbitmq::types::{Exchange, Queue};

/// An AMQP based publisher for the Batch distributed task queue.
#[derive(Clone)]
pub struct Publisher {
    channel: Channel<Stream>,
    heartbeat_handle: Arc<HeartbeatHandle>,
}

impl fmt::Debug for Publisher {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(f, "Publisher {{ }}")
    }
}

impl Publisher {
    /// Create a `Publisher` instance from a RabbitMQ URI and an explicit tokio handle.
    pub fn new_with_handle<E, Q>(
        connection_url: &str,
        exchanges_iter: E,
        queues_iter: Q,
        handle: Handle,
    ) -> Box<Future<Item = Self, Error = Error> + Send>
    where
        E: IntoIterator<Item = Exchange> + Send,
        Q: IntoIterator<Item = Queue> + Send,
    {
        let exchanges = exchanges_iter.into_iter().collect::<Vec<_>>();
        let queues = queues_iter.into_iter().collect::<Vec<_>>();

        let task = connect(connection_url, handle)
            .and_then(|(client, heartbeat_handle)| {
                trace!("Creating publisher's RabbitMQ channel");
                client
                    .create_channel()
                    .map(move |channel| {
                        trace!("Created publisher's RabbitMQ channel");
                        (channel, heartbeat_handle)
                    })
                    .map_err(|e| ErrorKind::Rabbitmq(e).into())
            })
            .and_then(move |(channel, heartbeat_handle)| {
                trace!("Declaring publisher's RabbitMQ exchanges");
                let channel_ = channel.clone();
                declare_exchanges(exchanges, channel_)
                    .map_err(|e| ErrorKind::Rabbitmq(e).into())
                    .map(|_| (channel, heartbeat_handle))
            })
            .and_then(move |(channel, heartbeat_handle)| {
                trace!("Declaring publisher's RabbitMQ queues");
                let channel_ = channel.clone();
                declare_queues(queues, channel_)
                    .map_err(|e| ErrorKind::Rabbitmq(e).into())
                    .map(|_| (channel, heartbeat_handle))
            })
            .map(move |(channel, heartbeat_handle)| Publisher {
                channel,
                heartbeat_handle: Arc::new(heartbeat_handle),
            });
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
    ) -> Box<Future<Item = (), Error = Error> + Send> {
        let task = self.channel
            .basic_publish(exchange, routing_key, serialized, options, properties)
            .and_then(move |_| future::ok(()))
            .map_err(|e| ErrorKind::Rabbitmq(e).into());
        Box::new(task)
    }
}

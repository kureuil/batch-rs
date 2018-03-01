//! `RabbitMQ` broker implementation

use std::collections::BTreeSet;
use std::cmp;
use std::fmt;
use std::io;
use std::iter::FromIterator;
use std::net::{self, ToSocketAddrs};
use std::result::Result as StdResult;
use std::thread;

use futures::{future, Async, Future, IntoFuture, Poll, Stream};
use lapin::channel::{BasicConsumeOptions, BasicProperties, BasicPublishOptions, Channel,
                     ExchangeBindOptions, ExchangeDeclareOptions, QueueBindOptions,
                     QueueDeclareOptions};
use lapin::client::{Client, ConnectionOptions};
use lapin::types::{AMQPValue, FieldTable};
use lapin_async::queue::Message;
use lapin_rustls::AMQPConnectionRustlsExt;
use lapin_tls_api::AMQPStream;
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpStream;

use de;
use error::{Error, ErrorKind};
use job::Job;
use ser;

/// Declare the given queues to the given `Channel`.
fn declare_queues<Q>(
    queues: Q,
    channel: Channel<AMQPStream>,
) -> Box<Future<Item = (), Error = io::Error>>
where
    Q: IntoIterator<Item = Queue> + 'static,
{
    let task = future::loop_fn(queues.into_iter(), move |mut iter| {
        let next = iter.next();
        let task: Box<Future<Item = future::Loop<_, _>, Error = io::Error>> =
            if let Some(queue) = next {
                let binding_channel = channel.clone();
                let task = channel
                    .queue_declare(&queue.name, &queue.options, &queue.arguments)
                    .and_then(move |_| {
                        future::join_all(queue.bindings().clone().into_iter().map(move |b| {
                            binding_channel.queue_bind(
                                queue.name(),
                                &b.exchange,
                                &b.routing_key,
                                &QueueBindOptions::default(),
                                &FieldTable::new(),
                            )
                        }))
                    })
                    .and_then(|_| Ok(future::Loop::Continue(iter)));
                Box::new(task)
            } else {
                Box::new(future::ok(future::Loop::Break(())))
            };
        task
    });
    Box::new(task.map(|_| ()))
}

/// Declare the given exchanges to the given `Channel`.
fn declare_exchanges<E>(
    exchanges: E,
    channel: Channel<AMQPStream>,
) -> Box<Future<Item = (), Error = io::Error>>
where
    E: IntoIterator<Item = Exchange> + 'static,
{
    let task = future::loop_fn(exchanges.into_iter(), move |mut iter| {
        let next = iter.next();
        let task: Box<Future<Item = future::Loop<_, _>, Error = io::Error>> =
            if let Some(exchange) = next {
                let binding_channel = channel.clone();
                let task = channel
                    .exchange_declare(
                        exchange.name(),
                        exchange.exchange_type(),
                        &exchange.options,
                        &exchange.arguments,
                    )
                    .and_then(move |_| {
                        future::join_all(exchange.bindings().clone().into_iter().map(move |b| {
                            binding_channel.exchange_bind(
                                &b.exchange,
                                &exchange.name,
                                &b.routing_key,
                                &ExchangeBindOptions::default(),
                                &FieldTable::new(),
                            )
                        }))
                    })
                    .and_then(|_| Ok(future::Loop::Continue(iter)));
                Box::new(task)
            } else {
                Box::new(future::ok(future::Loop::Break(())))
            };
        task
    });
    Box::new(task.map(|_| ()))
}

/// An AMQP based broker for the Batch distributed task queue.
#[derive(Clone)]
pub struct RabbitmqBroker {
    exchanges: Vec<Exchange>,
    queues: Vec<Queue>,
    publish_channel: Channel<AMQPStream>,
    client: Client<AMQPStream>,
}

impl fmt::Debug for RabbitmqBroker {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(
            f,
            "RabbitmqBroker {{ exchanges: {:?} queues: {:?} }}",
            self.exchanges, self.queues
        )
    }
}

impl RabbitmqBroker {
    /// Create a `RabbitmqBroker` instance from a RabbitMQ URI and an explicit tokio handle.
    pub fn new_with_handle<E, Q>(
        connection_url: &str,
        exchanges_iter: E,
        queues_iter: Q,
        handle: Handle,
    ) -> Box<Future<Item = Self, Error = Error>>
    where
        E: IntoIterator<Item = Exchange>,
        Q: IntoIterator<Item = Queue>,
    {
        let exchanges = exchanges_iter.into_iter().collect::<Vec<_>>();
        let exchanges_ = exchanges.clone();

        let queues = queues_iter.into_iter().collect::<Vec<_>>();
        let queues_ = queues.clone();

        let task = connection_url
            .connect(handle, |err| {
                error!(
                    "An error occured in the RabbitMQ heartbeat handler: {}",
                    err
                )
            })
            .and_then(|client| client.create_channel().join(future::ok(client)))
            .and_then(move |(channel, client)| {
                let channel_ = channel.clone();
                declare_exchanges(exchanges_, channel).map(|_| (channel_, client))
            })
            .and_then(move |(channel, client)| {
                let channel_ = channel.clone();
                declare_queues(queues_, channel).map(|_| (channel_, client))
            })
            .and_then(move |(publish_channel, client)| {
                future::ok(RabbitmqBroker {
                    client,
                    publish_channel,
                    queues,
                    exchanges,
                })
            })
            .map_err(|e| ErrorKind::Rabbitmq(e).into());
        Box::new(task)
    }

    /// Return a `Future` of a `Stream` of incoming jobs (see `Self::Stream`).
    ///
    /// This method consumes the current connection in order to avoid mixing publishing
    /// and consuming jobs on the same connection (which more often than not leads to issues).
    pub fn recv(self) -> Box<Future<Item = RabbitmqStream, Error = Error>> {
        let consumer_exchanges = self.exchanges.clone();
        let consumer_queues = self.queues.clone();
        let queues = self.queues.clone();
        let task = self.client
            .create_channel()
            .and_then(|channel| {
                let channel_ = channel.clone();
                declare_exchanges(consumer_exchanges, channel).map(|_| channel_)
            })
            .and_then(|channel| {
                let channel_ = channel.clone();
                declare_queues(consumer_queues, channel).map(|_| channel_)
            })
            .and_then(|channel| {
                let consumer_channel = channel.clone();
                future::join_all(queues.into_iter().map(move |queue| {
                    consumer_channel.basic_consume(
                        &queue.name,
                        &format!("batch-rs-consumer-{}", queue.name),
                        &BasicConsumeOptions::default(),
                        &FieldTable::new(),
                    )
                })).join(future::ok(channel))
            })
            .and_then(|(mut consumers, channel)| {
                let initial: Box<Stream<Item = Message, Error = io::Error> + Send> =
                    Box::new(consumers.pop().unwrap());
                let consumer = consumers
                    .into_iter()
                    .fold(initial, |acc, consumer| Box::new(acc.select(consumer)));
                future::ok(RabbitmqStream {
                    channel,
                    stream: consumer,
                })
            })
            .map_err(|e| ErrorKind::Rabbitmq(e).into());
        Box::new(task)
    }

    /// Send a job to the broker.
    ///
    /// Returns a `Future` that completes once the job is sent to the broker.
    pub fn send(
        &self,
        job: &Job,
        properties: BasicProperties,
    ) -> Box<Future<Item = (), Error = Error>> {
        let channel = self.publish_channel.clone();
        let serialized = match ser::to_vec(&job) {
            Ok(serialized) => serialized,
            Err(e) => return Box::new(future::err(ErrorKind::Serialization(e).into())),
        };
        let task = channel
            .basic_publish(
                "",
                job.queue(),
                &serialized,
                &BasicPublishOptions::default(),
                properties,
            )
            .and_then(move |_| future::ok(()))
            .map_err(|e| ErrorKind::Rabbitmq(e).into());
        Box::new(task)
    }
}

/// A `Consumer` of incoming jobs.
///
/// The type of the stream is a tuple containing a `u64` which is a unique ID for the
/// job used when `ack`'ing or `reject`'ing it, and a `Job` instance.
pub struct RabbitmqStream {
    channel: Channel<AMQPStream>,
    stream: Box<Stream<Item = Message, Error = io::Error> + Send>,
}

impl fmt::Debug for RabbitmqStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(f, "RabbitmqStream {{ }}")
    }
}

impl RabbitmqStream {
    /// Acknowledge the successful execution of a `Task`.
    ///
    /// Returns a `Future` that completes once the `ack` is sent to the broker.
    pub fn ack(&self, uid: u64) -> Box<Future<Item = (), Error = Error>> {
        let task = self.channel
            .basic_ack(uid)
            .map_err(|e| ErrorKind::Rabbitmq(e).into());
        Box::new(task)
    }

    /// Reject the successful execution of a `Task`.
    ///
    /// Returns a `Future` that completes once the `reject` is sent to the broker.
    pub fn reject(&self, uid: u64) -> Box<Future<Item = (), Error = Error>> {
        let task = self.channel
            .basic_reject(uid, false)
            .map_err(|e| ErrorKind::Rabbitmq(e).into());
        Box::new(task)
    }
}

impl Stream for RabbitmqStream {
    type Item = (u64, Job);
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let async = match self.stream.poll() {
            Ok(async) => async,
            Err(e) => return Err(ErrorKind::Rabbitmq(e).into()),
        };
        let option = match async {
            Async::Ready(option) => option,
            Async::NotReady => return Ok(Async::NotReady),
        };
        let message = match option {
            Some(message) => message,
            None => return Ok(Async::Ready(None)),
        };
        let job: Job = de::from_slice(&message.data).map_err(ErrorKind::Deserialization)?;
        Ok(Async::Ready(Some((message.delivery_tag, job))))
    }
}

/// A binding from a queue to an exchange, or from an exchange to an exchange.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binding {
    exchange: String,
    routing_key: String,
}

/// A `RabbitMQ` exchange.
#[derive(Clone, Debug)]
pub struct Exchange {
    name: String,
    exchange_type: String,
    bindings: BTreeSet<Binding>,
    options: ExchangeDeclareOptions,
    arguments: FieldTable,
}

impl Default for Exchange {
    fn default() -> Exchange {
        Exchange {
            name: "".into(),
            exchange_type: "direct".into(),
            bindings: BTreeSet::new(),
            options: ExchangeDeclareOptions::default(),
            arguments: FieldTable::new(),
        }
    }
}

impl cmp::PartialEq for Exchange {
    fn eq(&self, other: &Exchange) -> bool {
        self.name == other.name
    }
}

impl cmp::Eq for Exchange {}

impl cmp::PartialOrd for Exchange {
    fn partial_cmp(&self, other: &Exchange) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Exchange {
    fn cmp(&self, other: &Exchange) -> cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Exchange {
    /// Returns the name associated to this `Exchange`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the type associated to this `Exchange`.
    pub fn exchange_type(&self) -> &str {
        &self.exchange_type
    }

    /// Returns the bindings associated to this `Exchange`.
    pub(crate) fn bindings(&self) -> &BTreeSet<Binding> {
        &self.bindings
    }
}

/// A builder for `RabbitMQ` `Exchange`.
#[derive(Debug)]
pub struct ExchangeBuilder {
    name: String,
    bindings: BTreeSet<Binding>,
}

impl ExchangeBuilder {
    /// Create a new `ExchangeBuilder` instance from the desired exchange name.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let builder = ExchangeBuilder::new("batch.example");
    /// ```
    pub fn new(name: &str) -> ExchangeBuilder {
        ExchangeBuilder {
            name: name.into(),
            bindings: BTreeSet::new(),
        }
    }

    /// Binds this exchange to another exchange via a routing key.
    ///
    /// All of the messages posted to this exchange associated to the given routing key
    /// are automatically sent to the given exchange.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let builder = ExchangeBuilder::new("batch.example")
    ///     .bind("batch.messaging", "hello-world");
    /// ```
    pub fn bind(mut self, exchange: &str, routing_key: &str) -> Self {
        self.bindings.insert(Binding {
            exchange: exchange.into(),
            routing_key: routing_key.into(),
        });
        self
    }

    /// Build a new `Exchange` instance from this builder data.
    pub(crate) fn build(self) -> Exchange {
        Exchange {
            name: self.name,
            exchange_type: "direct".into(),
            bindings: self.bindings,
            options: ExchangeDeclareOptions::default(),
            arguments: FieldTable::new(),
        }
    }
}

/// Shorthand to create a new `ExchangeBuilder` instance.
pub fn exchange(name: &str) -> ExchangeBuilder {
    ExchangeBuilder::new(name)
}

/// A `RabbitMQ` queue.
#[derive(Clone, Debug)]
pub struct Queue {
    name: String,
    bindings: BTreeSet<Binding>,
    options: QueueDeclareOptions,
    arguments: FieldTable,
}

impl cmp::PartialEq for Queue {
    fn eq(&self, other: &Queue) -> bool {
        self.name == other.name
    }
}

impl cmp::Eq for Queue {}

impl cmp::PartialOrd for Queue {
    fn partial_cmp(&self, other: &Queue) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Queue {
    fn cmp(&self, other: &Queue) -> cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Queue {
    /// Returns the name associated to this `Queue`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the bindings associated to this `Queue`.
    pub(crate) fn bindings(&self) -> &BTreeSet<Binding> {
        &self.bindings
    }
}

/// A builder for `RabbitMQ` `Queue`.
#[derive(Debug)]
pub struct QueueBuilder {
    name: String,
    bindings: BTreeSet<Binding>,
    arguments: FieldTable,
}

impl QueueBuilder {
    /// Create a new `QueueBuilder` from the desired queue name.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let queue = QueueBuilder::new("video-transcoding");
    /// ```
    pub fn new(name: &str) -> QueueBuilder {
        QueueBuilder {
            name: name.into(),
            bindings: BTreeSet::new(),
            arguments: FieldTable::new(),
        }
    }

    /// Bind this queue to an exchange via a routing key.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// QueueBuilder::new("video-transcoding")
    ///     .bind("movies", "transcoding")
    ///     .bind("series", "transcoding")
    ///     .bind("anime", "transcoding");
    /// ```
    pub fn bind(mut self, exchange: &str, routing_key: &str) -> Self {
        self.bindings.insert(Binding {
            exchange: exchange.into(),
            routing_key: routing_key.into(),
        });
        self
    }

    /// Enable task priorities on this queue.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// QueueBuilder::new("video-transcoding")
    ///     .enable_priorities();
    /// ```
    pub fn enable_priorities(mut self) -> Self {
        self.arguments
            .insert("x-max-priority".to_string(), AMQPValue::ShortShortUInt(4));
        self
    }

    /// Create a new `Queue` instance from this builder data.
    pub(crate) fn build(self) -> Queue {
        Queue {
            name: self.name,
            bindings: self.bindings,
            options: QueueDeclareOptions::default(),
            arguments: self.arguments,
        }
    }
}

/// Shorthand to create a new `QueueBuilder` instance.
pub fn queue(name: &str) -> QueueBuilder {
    QueueBuilder::new(name)
}

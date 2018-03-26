//! `RabbitMQ` broker implementation

use std::collections::BTreeSet;
use std::cmp;
use std::fmt;
use std::io::{self, Read, Write};
use std::iter::FromIterator;
use std::net::{self, ToSocketAddrs};
use std::result::Result as StdResult;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use amq_protocol::uri::{AMQPScheme, AMQPUri};
use bytes::{Buf, BufMut};
use futures::{self, future, Async, Future, IntoFuture, Poll};
use lapin_async::generated::basic::Properties;
use lapin::channel::{BasicConsumeOptions, BasicProperties, BasicPublishOptions, Channel,
                     ExchangeBindOptions, ExchangeDeclareOptions, QueueBindOptions,
                     QueueDeclareOptions};
use lapin::client::{Client, ConnectionOptions};
use lapin::types::{self, AMQPValue, FieldTable};
use lapin_async::queue::Message;
use native_tls::TlsConnector;
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_tls::TlsConnectorExt;

use de;
use error::{Error, ErrorKind, Result};
use ser;

/// Declare the given queues to the given `Channel`.
fn declare_queues<Q>(
    queues: Q,
    channel: Channel<Stream>,
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
                    .queue_declare(queue.name(), queue.options(), queue.arguments())
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
    channel: Channel<Stream>,
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
                        exchange.kind(),
                        exchange.options(),
                        exchange.arguments(),
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

enum Stream {
    Raw(::tokio_core::net::TcpStream),
    Tls(::tokio_tls::TlsStream<::tokio_core::net::TcpStream>),
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            Stream::Raw(ref mut raw) => raw.read(buf),
            Stream::Tls(ref mut raw) => raw.read(buf),
        }
    }
}

impl AsyncRead for Stream {
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        match *self {
            Stream::Raw(ref raw) => raw.prepare_uninitialized_buffer(buf),
            Stream::Tls(ref raw) => raw.prepare_uninitialized_buffer(buf),
        }
    }

    fn read_buf<B: BufMut>(&mut self, buf: &mut B) -> Poll<usize, io::Error> {
        match *self {
            Stream::Raw(ref mut raw) => raw.read_buf(buf),
            Stream::Tls(ref mut raw) => raw.read_buf(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            Stream::Raw(ref mut raw) => raw.write(buf),
            Stream::Tls(ref mut raw) => raw.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match *self {
            Stream::Raw(ref mut raw) => raw.flush(),
            Stream::Tls(ref mut raw) => raw.flush(),
        }
    }
}

impl AsyncWrite for Stream {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        match *self {
            Stream::Raw(ref mut raw) => raw.shutdown(),
            Stream::Tls(ref mut raw) => raw.shutdown(),
        }
    }

    fn write_buf<B: Buf>(&mut self, buf: &mut B) -> Poll<usize, io::Error> {
        match *self {
            Stream::Raw(ref mut raw) => raw.write_buf(buf),
            Stream::Tls(ref mut raw) => raw.write_buf(buf),
        }
    }
}

fn connect(
    connection_url: &str,
    handle: Handle,
) -> Box<Future<Item = Client<Stream>, Error = Error>> {
    let heartbeat_handle = handle.clone();
    let task = AMQPUri::from_str(connection_url)
        .map_err(|e| ErrorKind::InvalidUrl(e).into())
        .into_future()
        .and_then(|uri| {
            let addr_uri = uri.clone();
            let addr = (addr_uri.authority.host.as_ref(), addr_uri.authority.port);
            net::TcpStream::connect(addr)
                .map_err(|e| ErrorKind::Io(e).into())
                .into_future()
                .join(future::ok(uri))
        })
        .and_then(move |(stream, uri)| {
            let task: Box<Future<Item = Stream, Error = Error>> = if uri.scheme == AMQPScheme::AMQP
            {
                let task = TcpStream::from_stream(stream, &handle)
                    .map(|stream| Stream::Raw(stream))
                    .map_err(|e| ErrorKind::Io(e).into())
                    .into_future();
                Box::new(task)
            } else {
                let host = uri.authority.host.clone();
                let task = TlsConnector::builder()
                    .map_err(|e| ErrorKind::Tls(e).into())
                    .into_future()
                    .and_then(|builder| builder.build().map_err(|e| ErrorKind::Tls(e).into()))
                    .and_then(move |connector| {
                        TcpStream::from_stream(stream, &handle)
                            .map_err(|e| ErrorKind::Io(e).into())
                            .into_future()
                            .join(future::ok(connector))
                    })
                    .and_then(move |(stream, connector)| {
                        connector
                            .connect_async(&host, stream)
                            .map(|stream| Stream::Tls(stream))
                            .map_err(|e| ErrorKind::Tls(e).into())
                    });
                Box::new(task)
            };
            task.join(future::ok(uri))
        })
        .and_then(move |(stream, uri)| {
            let opts = ConnectionOptions {
                username: uri.authority.userinfo.username,
                password: uri.authority.userinfo.password,
                vhost: uri.vhost,
                frame_max: uri.query.frame_max.unwrap_or(0),
                heartbeat: uri.query.heartbeat.unwrap_or(0),
            };
            Client::connect(stream, &opts)
                .map_err(|e| ErrorKind::Rabbitmq(e).into())
                .map(move |(client, heartbeat_fn)| {
                    let heartbeat_client = client.clone();
                    heartbeat_handle.spawn(
                        heartbeat_fn(&heartbeat_client)
                            .map_err(|e| eprintln!("RabbitMQ heartbeat error: {}", e)),
                    );
                    client
                })
        });
    Box::new(task)
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
            .map(move |(channel, client)| {
                Publisher {
                    client,
                    channel,
                }
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
    ) -> Box<Future<Item = (), Error = Error>> {
        let task = self.channel
            .basic_publish(exchange, routing_key, serialized, options, properties)
            .and_then(move |_| future::ok(()))
            .map_err(|e| ErrorKind::Rabbitmq(e).into());
        Box::new(task)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Properties")]
pub struct PropertiesDef {
    pub content_type: Option<types::ShortString>,
    pub content_encoding: Option<types::ShortString>,
    pub headers: Option<types::FieldTable>,
    pub delivery_mode: Option<types::ShortShortUInt>,
    pub priority: Option<types::ShortShortUInt>,
    pub correlation_id: Option<types::ShortString>,
    pub reply_to: Option<types::ShortString>,
    pub expiration: Option<types::ShortString>,
    pub message_id: Option<types::ShortString>,
    pub timestamp: Option<types::Timestamp>,
    pub type_: Option<types::ShortString>,
    pub user_id: Option<types::ShortString>,
    pub app_id: Option<types::ShortString>,
    pub cluster_id: Option<types::ShortString>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Message")]
pub struct MessageDef {
    pub delivery_tag: types::LongLongUInt,
    pub exchange: String,
    pub routing_key: String,
    pub redelivered: bool,
    #[serde(with = "PropertiesDef")]
    pub properties: Properties,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delivery(#[serde(with = "MessageDef")] pub Message);

impl Delivery {
    pub fn tag(&self) -> u64 {
        self.0.delivery_tag
    }

    pub fn task(&self) -> &str {
        self.0
            .properties
            .headers
            .as_ref()
            .map(|hdrs| match hdrs.get("task") {
                Some(&AMQPValue::LongString(ref task)) => task.as_ref(),
                _ => "",
            })
            .unwrap_or("")
    }

    pub fn task_id(&self) -> &str {
        self.0
            .properties
            .correlation_id
            .as_ref()
            .map_or("", String::as_ref)
    }

    pub fn exchange(&self) -> &str {
        &self.0.exchange
    }

    pub fn routing_key(&self) -> &str {
        &self.0.routing_key
    }

    pub fn data(&self) -> &[u8] {
        &self.0.data
    }

    pub fn properties(&self) -> &Properties {
        &self.0.properties
    }

    pub fn timeout(&self) -> (Option<Duration>, Option<Duration>) {
        self.0
            .properties
            .headers
            .as_ref()
            .map(|hdrs| match hdrs.get("timelimit") {
                Some(&AMQPValue::FieldArray(ref vec)) if vec.len() == 2 => {
                    let soft_limit = match vec[0] {
                        AMQPValue::Timestamp(s) => Some(Duration::from_secs(s)),
                        _ => None,
                    };
                    let hard_limit = match vec[1] {
                        AMQPValue::Timestamp(s) => Some(Duration::from_secs(s)),
                        _ => None,
                    };
                    (soft_limit, hard_limit)
                }
                _ => (None, None),
            })
            .unwrap_or((None, None))
    }

    pub fn retries(&self) -> u32 {
        self.0
            .properties
            .headers
            .as_ref()
            .map(|hdrs| match hdrs.get("retries") {
                Some(&AMQPValue::LongUInt(retries)) => retries,
                _ => 0,
            })
            .unwrap_or(0)
    }

    pub fn incr_retries(&mut self) -> u32 {
        let incrd_retries = self.retries() + 1;
        let mut headers = self.0
            .properties
            .headers
            .take()
            .unwrap_or_else(|| FieldTable::new());
        headers.insert("retries".to_string(), AMQPValue::LongUInt(incrd_retries));
        self.0.properties.headers = Some(headers);
        incrd_retries
    }

    pub fn should_retry(&mut self, max_retries: u32) -> bool {
        self.incr_retries() < max_retries
    }
}

/// A `Consumer` of incoming jobs.
///
/// The type of the stream is a tuple containing a `u64` which is a unique ID for the
/// job used when `ack`'ing or `reject`'ing it, and a `Job` instance.
pub struct Consumer {
    client: Client<Stream>,
    channel: Channel<Stream>,
    stream: Box<futures::Stream<Item = Message, Error = io::Error> + Send>,
}

impl fmt::Debug for Consumer {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(f, "Consumer {{ }}")
    }
}

impl Consumer {
    /// Create a `Consumer` instance from a RabbitMQ URI and an explicit tokio handle.
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
        let queues = queues_iter.into_iter().collect::<Vec<_>>();
        let queues_ = queues.clone();

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
            .and_then(move |(channel, client)| {
                let channel_ = channel.clone();
                declare_queues(queues_, channel_)
                    .map_err(|e| ErrorKind::Rabbitmq(e).into())
                    .map(|_| (channel, client))
            })
            .and_then(|(channel, client)| {
                let consumer_channel = channel.clone();
                future::join_all(queues.into_iter().map(move |queue| {
                    consumer_channel.basic_consume(
                        &queue.name,
                        &format!("batch-rs-consumer-{}", queue.name),
                        &BasicConsumeOptions::default(),
                        &FieldTable::new(),
                    ).map_err(|e| ErrorKind::Rabbitmq(e).into())
                })).join(future::ok((channel, client)))
            })
            .map(move |(mut consumers, (channel, client))| {
                let initial: Box<
                    futures::Stream<Item = Message, Error = io::Error> + Send,
                > = Box::new(consumers.pop().unwrap());
                let stream = consumers.into_iter().fold(initial, |acc, consumer| {
                    Box::new(futures::Stream::select(acc, consumer))
                });
                Consumer {
                    client,
                    channel,
                    stream,
                }
            });
        Box::new(task)
    }

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

impl futures::Stream for Consumer {
    type Item = Delivery;
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
        Ok(Async::Ready(Some(Delivery(message))))
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
    kind: String,
    bindings: BTreeSet<Binding>,
    options: ExchangeDeclareOptions,
    arguments: FieldTable,
}

impl Default for Exchange {
    fn default() -> Exchange {
        Exchange {
            name: "".into(),
            kind: "direct".into(),
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
    /// Return the name of this `Exchange`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the kind of this `Exchange`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Return the bindings associated to this `Exchange`.
    pub(crate) fn bindings(&self) -> &BTreeSet<Binding> {
        &self.bindings
    }

    /// Return the options of this `Exchange`.
    pub fn options(&self) -> &ExchangeDeclareOptions {
        &self.options
    }

    /// Return the arguments of this `Exchange`.
    pub fn arguments(&self) -> &FieldTable {
        &self.arguments
    }
}

/// A builder for `RabbitMQ` `Exchange`.
#[derive(Debug)]
pub struct ExchangeBuilder {
    name: String,
    bindings: BTreeSet<Binding>,
    options: ExchangeDeclareOptions,
    arguments: FieldTable,
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
            options: ExchangeDeclareOptions::default(),
            arguments: FieldTable::new(),
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

    /// Return a reference the declare options for this exchange.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let builder = ExchangeBuilder::new("batch.example");
    /// {
    ///     let options = builder.options();
    ///     println!("Options: {:?}", options);
    /// }
    /// ```
    pub fn options(&self) -> &ExchangeDeclareOptions {
        &self.options
    }

    /// Return a mutable reference to the declare options for this exchange.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let mut builder = ExchangeBuilder::new("batch.example");
    /// {
    ///     let options = builder.options_mut();
    ///     options.durable = true;
    ///     println!("Options: {:?}", options);
    /// }
    /// ```
    pub fn options_mut(&mut self) -> &mut ExchangeDeclareOptions {
        &mut self.options
    }

    /// Return a reference to the exchange arguments.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let builder = ExchangeBuilder::new("batch.example");
    /// {
    ///     let arguments = builder.arguments();
    ///     println!("Arguments: {:?}", arguments);
    /// }
    /// ```
    pub fn arguments(&self) -> &FieldTable {
        &self.arguments
    }

    /// Return a mutable reference to the exchange arguments.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate batch;
    /// extern crate lapin_futures;
    ///
    /// use lapin_futures::types::AMQPValue;
    /// use batch::ExchangeBuilder;
    ///
    /// # fn main() {
    /// let mut builder = ExchangeBuilder::new("batch.example");
    /// {
    ///     let arguments = builder.arguments_mut();
    ///     arguments.insert("x-custom-argument".to_string(), AMQPValue::Boolean(true));
    ///     println!("Arguments: {:?}", arguments);
    /// }
    /// # }
    /// ```
    pub fn arguments_mut(&mut self) -> &mut FieldTable {
        &mut self.arguments
    }

    /// Build a new `Exchange` instance from this builder data.
    pub(crate) fn build(self) -> Exchange {
        Exchange {
            name: self.name,
            kind: "direct".into(),
            bindings: self.bindings,
            options: self.options,
            arguments: self.arguments,
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
    /// Return the name of this `Queue`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the bindings associated to this `Queue`.
    pub(crate) fn bindings(&self) -> &BTreeSet<Binding> {
        &self.bindings
    }

    /// Return the options used when declaring this `Queue`.
    pub fn options(&self) -> &QueueDeclareOptions {
        &self.options
    }

    /// Return the arguments used when declaring this `Queue`.
    pub fn arguments(&self) -> &FieldTable {
        &self.arguments
    }
}

/// A builder for `RabbitMQ` `Queue`.
#[derive(Debug)]
pub struct QueueBuilder {
    name: String,
    bindings: BTreeSet<Binding>,
    options: QueueDeclareOptions,
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
            options: QueueDeclareOptions::default(),
            arguments: FieldTable::new(),
        }
    }

    /// Return a reference the declare options for this queue.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let builder = QueueBuilder::new("video-transcoding");
    /// {
    ///     let options = builder.options();
    ///     println!("Options: {:?}", options);
    /// }
    /// ```
    pub fn options(&self) -> &QueueDeclareOptions {
        &self.options
    }

    /// Return a mutable reference the declare options for this queue.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let mut builder = QueueBuilder::new("video-transcoding");
    /// {
    ///     let options = builder.options_mut();
    ///     options.auto_delete = true;
    ///     println!("Options: {:?}", options);
    /// }
    /// ```
    pub fn options_mut(&mut self) -> &mut QueueDeclareOptions {
        &mut self.options
    }

    /// Return a reference to the queue arguments.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let builder = QueueBuilder::new("video-transcoding");
    /// {
    ///     let arguments = builder.arguments();
    ///     println!("Arguments: {:?}", arguments);
    /// }
    /// ```
    pub fn arguments(&self) -> &FieldTable {
        &self.arguments
    }

    /// Return a mutable reference to the queue arguments.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate batch;
    /// extern crate lapin_futures;
    ///
    /// use lapin_futures::types::AMQPValue;
    /// use batch::QueueBuilder;
    ///
    /// # fn main() {
    /// let mut builder = QueueBuilder::new("video-transcoding");
    /// {
    ///     let arguments = builder.arguments_mut();
    ///     arguments.insert("x-custom-argument".to_string(), AMQPValue::Boolean(true));
    ///     println!("Arguments: {:?}", arguments);
    /// }
    /// # }
    /// ```
    pub fn arguments_mut(&mut self) -> &mut FieldTable {
        &mut self.arguments
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
            options: self.options,
            arguments: self.arguments,
        }
    }
}

/// Shorthand to create a new `QueueBuilder` instance.
pub fn queue(name: &str) -> QueueBuilder {
    QueueBuilder::new(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use task::Priority;

    #[test]
    fn priority_queue() {
        use std::collections::VecDeque;
        use futures::Stream;

        ::env_logger::init();
        let mut core = Core::new().unwrap();
        let ex = "batch.tests.priorities";
        let rk = "prioritised-hello";
        let jobs = vec![
            (("job-1", ex, rk, &[]), Priority::Normal),
            (("job-2", ex, rk, &[]), Priority::Critical),
            (("job-3", ex, rk, &[]), Priority::Trivial),
            (("job-4", ex, rk, &[]), Priority::High),
            (("job-5", ex, rk, &[]), Priority::Low),
        ];
        let expected = VecDeque::from(vec!["job-2", "job-4", "job-1", "job-5", "job-3"]);

        let conn_url = "amqp://localhost/%2f";
        let exchanges = vec![exchange(ex).build()];
        let queues = vec![
            queue("tests.priorities")
                .enable_priorities()
                .bind(ex, rk)
                .build(),
        ];
        let handle = core.handle();
        let task = Publisher::new_with_handle(
            conn_url,
            exchanges.clone(),
            handle.clone(),
        ).and_then(|broker| {
            let tasks = jobs.iter().map(move |&(ref job, ref priority)| {
                let mut headers = FieldTable::new();
                headers.insert("lang".to_string(), AMQPValue::LongString("rs".to_string()));
                headers.insert("task".to_string(), AMQPValue::LongString(job.0.to_string()));
                let properties = BasicProperties {
                    priority: Some(priority.to_u8()),
                    headers: Some(headers),
                    ..Default::default()
                };
                broker.send(
                    job.1,
                    job.2,
                    job.3,
                    &BasicPublishOptions::default(),
                    properties,
                )
            });
            future::join_all(tasks)
        })
            .and_then(move |_| Consumer::new_with_handle(conn_url, exchanges, queues, handle))
            .and_then(|consumer| {
                println!("Created consumer: {:?}", consumer);
                future::loop_fn(
                    (consumer.into_future(), expected.clone()),
                    |(f, mut order)| {
                        println!("Start iterating over consumer: {:?} // {:?}", f, order);
                        f.map_err(|(e, _)| e)
                            .and_then(move |(next, consumer)| {
                                println!("Got delivery: {:?}", next);
                                let head = order.pop_front().unwrap();
                                let tail = order;
                                let delivery = next.unwrap();
                                println!("Comparing: {:?} to {:?}", delivery.task(), head);
                                assert_eq!(delivery.task(), head);
                                consumer.ack(delivery.tag()).map(|_| (consumer, tail))
                            })
                            .and_then(|(consumer, order)| {
                                if order.is_empty() {
                                    Ok(future::Loop::Break(()))
                                } else {
                                    Ok(future::Loop::Continue((consumer.into_future(), order)))
                                }
                            })
                    },
                )
            });
        core.run(task).unwrap();
    }
}

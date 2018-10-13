use amq_protocol::uri::AMQPUri;
use batch::{self, Dispatch};
use failure::Error;
use futures::sync::mpsc;
use futures::{future, sink, Async, Future, IntoFuture, Poll, Sink, Stream};
use lapin::channel::{
    BasicConsumeOptions, BasicProperties, BasicPublishOptions, Channel, ExchangeDeclareOptions,
    QueueBindOptions, QueueDeclareOptions,
};
use lapin::client::{self, Client, ConnectionOptions};
use lapin::consumer;
use lapin::message;
use lapin::queue::Queue;
use lapin::types::{AMQPValue, FieldTable};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::io;
use std::iter::FromIterator;
use std::str::FromStr;
use std::sync::Arc;
use tokio_executor::spawn;

use consumer::Consumer;
use declare::Declare;
use queue;
use stream;

mod sealed {
    use batch;
    use failure::Error;
    use futures::future;
    use serde::{Deserialize, Serialize};

    /// Stub job used to trick the type system in `Connection::declare`.
    #[derive(Debug, Deserialize, Serialize)]
    pub struct StubJob;

    impl batch::Job for StubJob {
        const NAME: &'static str = "";

        type PerformFuture = future::FutureResult<(), Error>;

        fn perform(self, _ctx: &batch::Factory) -> Self::PerformFuture {
            future::ok(())
        }
    }
}

use self::sealed::StubJob;

fn amqp_properties(properties: &batch::Properties) -> BasicProperties {
    let mut headers = FieldTable::new();
    headers.insert("lang".into(), AMQPValue::LongString("rs".into()));
    headers.insert(
        "task".into(),
        AMQPValue::LongString(properties.task.clone()),
    );
    headers.insert("root_id".into(), AMQPValue::Void);
    headers.insert("parent_id".into(), AMQPValue::Void);
    headers.insert("group".into(), AMQPValue::Void);
    BasicProperties::default()
        .with_content_type("application/json".to_string())
        .with_content_encoding("utf-8".to_string())
        .with_correlation_id(properties.id.hyphenated().to_string())
        .with_headers(headers)
}

/// Builder for RabbitMQ `Connection`.
///
/// You can obtain an instance of this builder via the [`Connection::build`] method.
#[derive(Debug)]
pub struct Builder<'u> {
    uri: &'u str,
    pub(crate) queues: BTreeMap<String, queue::Queue>,
}

impl<'u> Builder<'u> {
    pub(crate) fn new(uri: &'u str) -> Self {
        Builder {
            uri,
            queues: Default::default(),
        }
    }

    /// Declare a queue.
    ///
    /// The given queue is not declared immediately but registered to be declared once we will
    /// connect to the RabbitMQ server.
    ///
    /// **Note**: This function takes a function taking a `StubJob` (deliberately not exposed) as
    /// parameter and returning a `Query`. The only type of importance here is the `Queue` type
    /// associated to the `Query`. We're using this syntax to circumvent the turbofish syntax.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use batch_rabbitmq::{queues};
    ///
    /// queues! {
    ///     Transcoding {
    ///         name = "transcoding"
    ///     }
    /// }
    ///
    /// fn example() {
    ///     // ...
    ///     let builder = Connection::build("amqp://guest:guest@localhost:5672/%2f")
    ///         .declare(Transcoding);
    ///     // ...
    /// }
    /// ```
    pub fn declare<Q>(mut self, _ctor: impl Fn(StubJob) -> batch::Query<StubJob, Q>) -> Builder<'u>
    where
        Q: batch::Queue + Declare,
    {
        Q::declare(&mut self);
        self
    }

    /// Connect to the RabbitMQ server & declare registered resources.
    ///
    /// This method consumes the builder instance and returns a [`Connect`] future.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate batch_rabbitmq;
    /// # extern crate tokio;
    /// #
    /// use batch_rabbitmq::Connection;
    /// use tokio::prelude::Future;
    ///
    /// let fut = Connection::build("amqp://guest:guest@localhost:5672/%2f")
    ///     .connect();
    ///
    /// # if false {
    /// tokio::run(
    ///     fut.map(|_| ())
    ///         .map_err(|e| eprintln!("An error occured: {}", e))
    /// );
    /// # }
    /// ```
    pub fn connect(self) -> ConnectFuture {
        let queues = self.queues;
        let queues2 = queues.clone();
        let fut = AMQPUri::from_str(self.uri)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e).into())
            .into_future()
            .and_then(|uri| stream::Stream::new(uri.clone()).map(|s| (s, uri)))
            .and_then(move |(stream, uri)| {
                trace!("Connecting to RabbitMQ broker");
                let opts = ConnectionOptions {
                    username: uri.authority.userinfo.username,
                    password: uri.authority.userinfo.password,
                    vhost: uri.vhost,
                    frame_max: uri.query.frame_max.unwrap_or(0),
                    heartbeat: uri.query.heartbeat.unwrap_or(0),
                };
                Client::connect(stream, opts).map_err(|e| e.into())
            }).and_then(move |(client, mut heartbeat)| {
                let handle = HeartbeatHandle(heartbeat.handle());
                trace!("Spawning RabbitMQ heartbeat future");
                spawn(heartbeat.map_err(|e| {
                    error!("Couldn't send heartbeat to RabbitMQ: {}", e);
                }));
                client
                    .create_channel()
                    .map(|channel| (client, channel, handle))
                    .map_err(Error::from)
            }).and_then(move |(client, channel, handle)| {
                let channel2 = channel.clone();
                let tasks: Vec<_> = queues2
                    .into_iter()
                    .map(move |(_, queue)| {
                        let task = channel2
                            .exchange_declare(
                                queue.exchange().name(),
                                queue.exchange().kind().as_ref(),
                                ExchangeDeclareOptions::default(),
                                FieldTable::new(),
                            ).map(|_| ())
                            .map_err(Error::from);
                        Box::new(task) as Box<Future<Item = (), Error = Error> + Send>
                    }).collect();
                future::join_all(tasks).map(move |_| (client, channel, handle))
            }).map(move |(client, channel, handle)| {
                let (publisher, consumer) = mpsc::channel(1024);
                let publish_channel = channel.clone();
                let background = consumer.for_each(move |dispatch: Dispatch| {
                    publish_channel
                        .basic_publish(
                            dispatch.destination(),
                            &dispatch.properties().task,
                            dispatch.payload().to_vec(),
                            BasicPublishOptions::default(),
                            amqp_properties(dispatch.properties()),
                        ).map(|_| ())
                        .map_err(|e| error!("Couldn't publish message to channel: {}", e))
                });
                spawn(background);
                let inner = Inner {
                    _channel: channel,
                    _handle: handle,
                    client,
                    publisher,
                    queues,
                };
                Connection {
                    inner: Arc::new(inner),
                }
            });
        ConnectFuture(Box::new(fut))
    }
}

/// The future returned by [`Builder::connect`].
#[must_use = "futures do nothing unless polled"]
pub struct ConnectFuture(Box<dyn Future<Item = Connection, Error = Error> + Send>);

impl fmt::Debug for ConnectFuture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ConnectFuture").finish()
    }
}

impl Future for ConnectFuture {
    type Item = Connection;

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

/// Connection to the RabbitMQ server.
///
/// Because this type doesn't interact directly with the network socket, it is safe to use
/// concurrently from multiple threads, although the [`batch::Client`] trait imposes a mutable
/// borrow publish a job. As mandated by the [`batch::Client`] trait, cloning this type has been
/// made as cheap as possible.
#[derive(Clone)]
pub struct Connection {
    inner: Arc<Inner>,
}

struct Inner {
    client: Client<stream::Stream>,
    publisher: mpsc::Sender<Dispatch>,
    queues: BTreeMap<String, queue::Queue>,
    _channel: Channel<stream::Stream>,
    _handle: HeartbeatHandle,
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Connection").finish()
    }
}

impl Connection {
    /// Create a new [`Builder`] for this connection.
    ///
    /// The given URI must follow RabbitMQ's [URI specification].
    ///
    /// [URI specification]: https://www.rabbitmq.com/uri-spec.html
    ///
    /// # Example
    ///
    /// ```
    /// use batch_rabbitmq::Connection;
    ///
    /// let builder = Connection::build("amqp://guest:guest@localhost:5672/%2f");
    /// ```
    pub fn build<'u>(uri: &'u str) -> Builder<'u> {
        Builder::new(uri)
    }

    /// Connects to the server at the given URI.
    ///
    /// The given URI must follow RabbitMQ's [URI specification].
    ///
    /// [URI specification]: https://www.rabbitmq.com/uri-spec.html
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate batch_rabbitmq;
    /// # extern crate tokio;
    /// use batch_rabbitmq::Connection;
    /// use tokio::prelude::Future;
    ///
    /// let fut = Connection::open("amqp://guest:guest@localhost:5672/%2f");
    /// # if false {
    /// tokio::run(
    ///     fut.map(|_| ())
    ///         .map_err(|e| eprintln!("An error occured: {}", e))
    /// );
    /// # }
    /// ```
    pub fn open(uri: &str) -> ConnectFuture {
        Builder::new(uri).connect()
    }
}

impl batch::Client for Connection {
    type SendFuture = SendFuture;

    fn send(&mut self, dispatch: batch::Dispatch) -> Self::SendFuture {
        SendFuture(self.inner.publisher.clone().send(dispatch))
    }

    type Consumer = Consumer;

    type ToConsumerFuture = Box<Future<Item = Self::Consumer, Error = Error> + Send>;

    /// Creates a consumer fetching messages from the given queues.
    ///
    /// # Panics
    ///
    /// This function will panic if the given iterator yield no items.
    fn to_consumer(
        &mut self,
        queues: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self::ToConsumerFuture {
        let names: HashSet<String> =
            HashSet::from_iter(queues.into_iter().map(|q| q.as_ref().to_string()));
        if names.len() == 0 {
            panic!("you must give a list of queues to consume from");
        }
        let queues: Vec<(String, queue::Queue)> = self
            .inner
            .queues
            .clone()
            .into_iter()
            .filter(|(k, _v)| names.contains(k))
            .collect();
        let task = self
            .inner
            .client
            .create_channel()
            .map_err(Error::from)
            .and_then(move |channel| {
                let tasks: Vec<_> = queues
                    .into_iter()
                    .map(|(name, queue)| {
                        let task = channel
                            .exchange_declare(
                                queue.exchange().name(),
                                queue.exchange().kind().as_ref(),
                                ExchangeDeclareOptions::default(),
                                FieldTable::new(),
                            ).map(|_| (name, queue))
                            .map_err(Error::from);
                        Box::new(task)
                            as Box<Future<Item = (String, queue::Queue), Error = Error> + Send>
                    }).collect();
                future::join_all(tasks).map(|queues| (channel, queues))
            }).and_then(move |(channel, queues)| {
                let tasks: Vec<_> = queues
                    .iter()
                    .map(|(_, queue)| {
                        let task = channel
                            .queue_declare(
                                queue.name(),
                                QueueDeclareOptions::default(),
                                FieldTable::new(),
                            ).map_err(Error::from);
                        Box::new(task) as Box<Future<Item = Queue, Error = Error> + Send>
                    }).collect();
                future::join_all(tasks).map(|declared| (channel, queues, declared))
            }).and_then(move |(channel, queues, declared)| {
                let mut tasks = vec![];
                for (_, queue) in queues {
                    for job in queue.callbacks().map(|(k, _v)| k) {
                        let task = channel
                            .queue_bind(
                                queue.name(),
                                queue.exchange().name(),
                                job,
                                QueueBindOptions::default(),
                                FieldTable::new(),
                            ).map_err(Error::from);
                        let boxed = Box::new(task) as Box<Future<Item = (), Error = Error> + Send>;
                        tasks.push(boxed);
                    }
                }
                future::join_all(tasks).map(move |_| (channel, declared))
            }).and_then(|(channel, declared)| {
                let tasks: Vec<_> = declared
                    .iter()
                    .map(|queue| {
                        let task = channel
                            .basic_consume(
                                &queue,
                                "", // We let RabbitMQ generate the consumer tag
                                BasicConsumeOptions::default(),
                                FieldTable::new(),
                            ).map_err(Error::from);
                        Box::new(task)
                            as Box<
                                Future<Item = consumer::Consumer<stream::Stream>, Error = Error>
                                    + Send,
                            >
                    }).collect();
                future::join_all(tasks).map(|consumers| (channel, consumers))
            }).and_then(|(channel, mut consumers)| {
                let combined: Box<
                    Stream<Item = message::Delivery, Error = Error> + Send + 'static,
                > = Box::new(consumers.pop().unwrap().map_err(Error::from));
                future::loop_fn((combined, consumers), |(combined, mut consumers)| {
                    let stream = match consumers.pop() {
                        None => return Ok(future::Loop::Break(combined)),
                        Some(stream) => stream,
                    };
                    let combined = Box::new(combined.select(stream.map_err(Error::from)));
                    Ok(future::Loop::Continue((combined, consumers)))
                }).map(move |combined| Consumer::new(channel, combined))
            });
        Box::new(task)
    }
}

/// The future returned when sending a dispatch to the broker.
#[derive(Debug)]
pub struct SendFuture(sink::Send<mpsc::Sender<Dispatch>>);

impl Future for SendFuture {
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.poll() {
            Ok(Async::Ready(_)) => Ok(Async::Ready(())),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(Error::from(e)),
        }
    }
}

struct HeartbeatHandle(Option<client::HeartbeatHandle>);

impl fmt::Debug for HeartbeatHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HeartbeatHandle").finish()
    }
}

impl Drop for HeartbeatHandle {
    fn drop(&mut self) {
        trace!("Signaling RabbitMQ heartbeat future to stop");
        if let Some(handle) = self.0.take() {
            handle.stop();
        } else {
            warn!("Couldn't acquire heartbeat handle");
        }
    }
}

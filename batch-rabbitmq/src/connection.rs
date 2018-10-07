use amq_protocol::uri::AMQPUri;
use batch::{self, Dispatch};
use failure::Error;
use futures::sync::mpsc::{channel, Sender};
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

    pub fn declare<Q>(mut self, _ctor: impl Fn(StubJob) -> batch::Query<StubJob, Q>) -> Builder<'u>
    where
        Q: batch::Queue + Declare,
    {
        // TODO: declare queue's exchange
        Q::declare(&mut self);
        self
    }

    pub fn connect(self) -> impl Future<Item = Connection, Error = Error> {
        let queues = self.queues;
        AMQPUri::from_str(self.uri)
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
                let (publisher, consumer) = channel(1024);
                client
                    .create_channel()
                    .map_err(|e| e.into())
                    .map(|channel| {
                        let consume_channel = channel.clone();
                        let background = consumer.for_each(move |dispatch: Dispatch| {
                            consume_channel
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
                    })
            })
    }
}

/// Connection to the RabbitMQ server.
#[derive(Clone)]
pub struct Connection {
    inner: Arc<Inner>,
}

struct Inner {
    client: Client<stream::Stream>,
    publisher: Sender<Dispatch>,
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
    pub fn build<'u>(uri: &'u str) -> Builder<'u> {
        Builder::new(uri)
    }

    pub fn open(uri: &str) -> impl Future<Item = Connection, Error = Error> {
        Builder::new(uri).connect()
    }
}

impl batch::ToConsumer for Connection {
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
pub struct SendFuture(sink::Send<Sender<Dispatch>>);

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

impl batch::Client for Connection {
    type SendFuture = SendFuture;

    fn send(&mut self, dispatch: batch::Dispatch) -> Self::SendFuture {
        SendFuture(self.inner.publisher.clone().send(dispatch))
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

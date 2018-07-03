use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::io;
use std::iter::FromIterator;
use std::net;
use std::str::FromStr;

use amq_protocol::uri::{AMQPScheme, AMQPUri};
use batch_core as batch;
use failure::Error;
use futures::sync::mpsc::{channel, Sender};
use futures::{future, Future, IntoFuture, Stream};
use lapin::channel::{
    BasicConsumeOptions, BasicPublishOptions, Channel, ExchangeDeclareOptions, QueueBindOptions,
    QueueDeclareOptions,
};
use lapin::client::{self, Client, ConnectionOptions};
use lapin::consumer;
use lapin::message;
use lapin::queue::Queue;
use lapin::types::FieldTable;
use native_tls::TlsConnector;
use tokio_executor::spawn;
use tokio_reactor::Handle;
use tokio_tcp::TcpStream;
use tokio_tls::TlsConnectorExt;

use consumer::Consumer;
use dispatch::Dispatch;
use exchange;
use queue;
use stream;

/// Connection to the RabbitMQ server.
pub struct Connection {
    inner: Inner,
}

struct Inner {
    client: Client<stream::Stream>,
    channel: Channel<stream::Stream>,
    publisher: Sender<Dispatch>,
    queues: BTreeMap<String, Vec<queue::Binding>>,
    _handle: HeartbeatHandle,
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Connection").finish()
    }
}

impl Connection {
    pub fn open(uri: &str) -> impl Future<Item = Connection, Error = Error> {
        AMQPUri::from_str(uri)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e).into())
            .into_future()
            .and_then(|uri| {
                trace!("Establishing TCP connection");
                let addr_uri = uri.clone();
                let addr = (addr_uri.authority.host.as_ref(), addr_uri.authority.port);
                net::TcpStream::connect(addr)
                    .map_err(|e| e.into())
                    .into_future()
                    .join(future::ok(uri))
            })
            .and_then(move |(stream, uri)| {
                let task: Box<Future<Item = stream::Stream, Error = Error> + Send> =
                    if uri.scheme == AMQPScheme::AMQP {
                        trace!("Wrapping TCP connection into tokio-tcp");
                        let handle = Handle::current();
                        let task = TcpStream::from_std(stream, &handle)
                            .map(stream::Stream::Raw)
                            .map_err(|e| e.into())
                            .into_future();
                        Box::new(task)
                    } else {
                        trace!("Wrapping TCP connection into tokio-tls");
                        let host = uri.authority.host.clone();
                        let task = TlsConnector::builder()
                            .map_err(|e| e.into())
                            .into_future()
                            .and_then(|builder| builder.build().map_err(|e| e.into()))
                            .and_then(move |connector| {
                                let handle = Handle::current();
                                TcpStream::from_std(stream, &handle)
                                    .map_err(|e| e.into())
                                    .into_future()
                                    .join(future::ok(connector))
                            })
                            .and_then(move |(stream, connector)| {
                                connector
                                    .connect_async(&host, stream)
                                    .map(stream::Stream::Tls)
                                    .map_err(|e| e.into())
                            });
                        Box::new(task)
                    };
                task.join(future::ok(uri))
            })
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
            })
            .and_then(move |(client, mut heartbeat)| {
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
                                    dispatch.exchange(),
                                    &dispatch.properties().task,
                                    dispatch.payload().to_vec(),
                                    BasicPublishOptions::default(),
                                    dispatch.to_amqp_properties(),
                                )
                                .map(|_| ())
                                .map_err(|e| error!("Couldn't publish message to channel: {}", e))
                        });
                        spawn(background);
                        Connection {
                            inner: Inner {
                                _handle: handle,
                                client,
                                channel,
                                publisher,
                                queues: BTreeMap::new(),
                            },
                        }
                    })
            })
    }
}

impl batch::Declarator<exchange::Builder, exchange::Exchange> for Connection {
    type DeclareFuture = Box<Future<Item = exchange::Exchange, Error = Error> + Send>;

    fn declare(&mut self, builder: exchange::Builder) -> Self::DeclareFuture {
        let publisher = self.inner.publisher.clone();
        let task = self
            .inner
            .channel
            .exchange_declare(
                &builder.name,
                builder.kind.as_ref(),
                ExchangeDeclareOptions::default(),
                FieldTable::new(),
            )
            .map(|_| exchange::Exchange::new(publisher, builder.name, builder.kind))
            .map_err(|e| e.into());
        Box::new(task)
    }
}

impl batch::Declarator<queue::Builder, queue::Queue> for Connection {
    type DeclareFuture = Box<Future<Item = queue::Queue, Error = Error> + Send>;

    fn declare(&mut self, builder: queue::Builder) -> Self::DeclareFuture {
        let queue = match builder.build() {
            Ok(q) => q,
            Err(e) => return Box::new(future::err(e)),
        };
        self.inner
            .queues
            .insert(queue.name().into(), queue.bindings.clone());
        Box::new(future::ok(queue))
    }
}

impl batch::ToConsumer for Connection {
    type Consumer = Consumer;

    type ToConsumerFuture = Box<Future<Item = Self::Consumer, Error = Error> + Send>;

    fn to_consumer(
        &mut self,
        queues: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self::ToConsumerFuture {
        let names: HashSet<String> =
            HashSet::from_iter(queues.into_iter().map(|q| q.as_ref().to_string()));
        let names: Vec<String> = self
            .inner
            .queues
            .keys()
            .filter(|q| names.contains(q.as_str()))
            .map(ToString::to_string)
            .collect();
        let queues = self.inner.queues.clone();
        let task = self
            .inner
            .client
            .create_channel()
            .map_err(Error::from)
            .and_then(move |channel| {
                let tasks: Vec<Box<Future<Item = Queue, Error = Error> + Send>> = names
                    .iter()
                    .map(|queue_name| {
                        let task = channel
                            .queue_declare(
                                &queue_name,
                                QueueDeclareOptions::default(),
                                FieldTable::new(),
                            )
                            .map_err(Error::from);
                        let boxed: Box<
                            Future<Item = Queue, Error = Error> + Send,
                        > = Box::new(task);
                        boxed
                    })
                    .collect();
                future::join_all(tasks).map(|names| (names, channel))
            })
            .and_then(move |(names, channel)| {
                let mut bindings = vec![];
                for (name, binding) in queues {
                    bindings.extend(binding.into_iter().map(|binding| (name.clone(), binding)));
                }
                let tasks: Vec<Box<Future<Item = (), Error = Error> + Send>> = bindings
                    .into_iter()
                    .map(|(queue, binding)| {
                        let task = channel
                            .queue_bind(
                                &queue,
                                &binding.exchange,
                                &binding.routing_key,
                                QueueBindOptions::default(),
                                FieldTable::new(),
                            )
                            .map_err(Error::from);
                        let boxed: Box<
                            Future<Item = (), Error = Error> + Send,
                        > = Box::new(task);
                        boxed
                    })
                    .collect();
                future::join_all(tasks).map(move |_| (names, channel))
            })
            .and_then(|(names, channel)| {
                let tasks: Vec<
                    Box<Future<Item = consumer::Consumer<stream::Stream>, Error = Error> + Send>,
                > = names
                    .iter()
                    .map(|queue_name| {
                        let task = channel
                            .basic_consume(
                                &queue_name,
                                "",
                                BasicConsumeOptions::default(),
                                FieldTable::new(),
                            )
                            .map_err(Error::from);
                        let boxed: Box<
                            Future<Item = consumer::Consumer<stream::Stream>, Error = Error> + Send,
                        > = Box::new(task);
                        boxed
                    })
                    .collect();
                future::join_all(tasks).map(|consumers| (consumers, channel))
            })
            .and_then(|(mut consumers, channel)| {
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

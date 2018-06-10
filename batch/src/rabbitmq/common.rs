use std::io;
use std::net;
use std::str::FromStr;

use amq_protocol::uri::{AMQPScheme, AMQPUri};
use futures::{future, Future, IntoFuture};
use lapin::channel::{Channel, ExchangeBindOptions, QueueBindOptions};
use lapin::client::{self, Client, ConnectionOptions};
use lapin::types::FieldTable;
use native_tls::TlsConnector;
use tokio_executor;
use tokio_reactor::Handle;
use tokio_tcp::TcpStream;
use tokio_tls::TlsConnectorExt;

use error::{Error, ErrorKind};
use rabbitmq::stream::Stream;
use rabbitmq::types::{Exchange, Queue};

/// Declare the given queues to the given `Channel`.
pub fn declare_queues<Q>(
    queues: Q,
    channel: Channel<Stream>,
) -> Box<Future<Item = (), Error = io::Error> + Send>
where
    Q: IntoIterator<Item = Queue> + 'static,
    Q::IntoIter: Send,
{
    let task = future::loop_fn(queues.into_iter(), move |mut iter| {
        let next = iter.next();
        let task: Box<Future<Item = future::Loop<_, _>, Error = io::Error> + Send> =
            if let Some(queue) = next {
                trace!("Declaring queue {:?}", queue.name());
                let binding_channel = channel.clone();
                let task = channel
                    .queue_declare(queue.name(), queue.options().clone(), queue.arguments().clone())
                    .and_then(move |_| {
                        future::join_all(queue.bindings().clone().into_iter().map(move |b| {
                            trace!(
                                "Binding queue {:?} to exchange {:?} on routing key {:?}",
                                queue.name(),
                                b.exchange(),
                                b.routing_key()
                            );
                            binding_channel.queue_bind(
                                queue.name(),
                                b.exchange(),
                                b.routing_key(),
                                QueueBindOptions::default(),
                                FieldTable::new(),
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
pub fn declare_exchanges<E>(
    exchanges: E,
    channel: Channel<Stream>,
) -> Box<Future<Item = (), Error = io::Error> + Send>
where
    E: IntoIterator<Item = Exchange> + 'static,
    E::IntoIter: Send,
{
    let task = future::loop_fn(exchanges.into_iter(), move |mut iter| {
        let next = iter.next();
        let task: Box<Future<Item = future::Loop<_, _>, Error = io::Error> + Send> =
            if let Some(exchange) = next {
                let binding_channel = channel.clone();
                trace!(
                    "Declaring RabbitMQ exchange {:?} ({:?})",
                    exchange.name(),
                    exchange.kind()
                );
                let task = channel
                    .exchange_declare(
                        exchange.name(),
                        exchange.kind(),
                        exchange.options().clone(),
                        exchange.arguments().clone(),
                    )
                    .and_then(move |_| {
                        future::join_all(exchange.bindings().clone().into_iter().map(move |b| {
                            trace!(
                                "Binding exchange {:?} ({:?}) to exchange {:?} on routing key {:?}",
                                exchange.name(),
                                exchange.kind(),
                                b.exchange(),
                                b.routing_key()
                            );
                            binding_channel.exchange_bind(
                                b.exchange(),
                                exchange.name(),
                                b.routing_key(),
                                ExchangeBindOptions::default(),
                                FieldTable::new(),
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

pub fn connect(
    connection_url: &str,
    handle: Handle,
) -> Box<Future<Item = (Client<Stream>, HeartbeatHandle), Error = Error> + Send> {
    let task = AMQPUri::from_str(connection_url)
        .map_err(|e| ErrorKind::InvalidUrl(e).into())
        .into_future()
        .and_then(|uri| {
            trace!("Establishing TCP connection");
            let addr_uri = uri.clone();
            let addr = (addr_uri.authority.host.as_ref(), addr_uri.authority.port);
            net::TcpStream::connect(addr)
                .map_err(|e| ErrorKind::Io(e).into())
                .into_future()
                .join(future::ok(uri))
        })
        .and_then(move |(stream, uri)| {
            let task: Box<Future<Item = Stream, Error = Error> + Send> =
                if uri.scheme == AMQPScheme::AMQP {
                    trace!("Wrapping TCP connection into tokio-tcp");
                    let task = TcpStream::from_std(stream, &handle)
                        .map(Stream::Raw)
                        .map_err(|e| ErrorKind::Io(e).into())
                        .into_future();
                    Box::new(task)
                } else {
                    trace!("Wrapping TCP connection into tokio-tls");
                    let host = uri.authority.host.clone();
                    let task = TlsConnector::builder()
                        .map_err(|e| ErrorKind::Tls(e).into())
                        .into_future()
                        .and_then(|builder| builder.build().map_err(|e| ErrorKind::Tls(e).into()))
                        .and_then(move |connector| {
                            TcpStream::from_std(stream, &handle)
                                .map_err(|e| ErrorKind::Io(e).into())
                                .into_future()
                                .join(future::ok(connector))
                        })
                        .and_then(move |(stream, connector)| {
                            connector
                                .connect_async(&host, stream)
                                .map(Stream::Tls)
                                .map_err(|e| ErrorKind::Tls(e).into())
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
            Client::connect(stream, opts)
                .map_err(|e| ErrorKind::Rabbitmq(e).into())
                .map(move |(client, mut heartbeat)| {
                    let heartbeat_handle = HeartbeatHandle(heartbeat.handle());
                    trace!("Spawning RabbitMQ heartbeat future");
                    tokio_executor::spawn(heartbeat.map_err(|e| {
                        error!("Couldn't send heartbeat to RabbitMQ: {}", e);
                    }));
                    (client, heartbeat_handle)
                })
        });
    Box::new(task)
}

pub struct HeartbeatHandle(Option<client::HeartbeatHandle>);

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

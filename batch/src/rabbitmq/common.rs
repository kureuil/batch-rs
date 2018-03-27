use std::io;
use std::net;
use std::str::FromStr;

use amq_protocol::uri::{AMQPScheme, AMQPUri};
use futures::{future, Future, IntoFuture};
use lapin::channel::{Channel, ExchangeBindOptions, QueueBindOptions};
use lapin::client::{Client, ConnectionOptions};
use lapin_async::types::FieldTable;
use native_tls::TlsConnector;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_tls::TlsConnectorExt;

use error::{Error, ErrorKind};
use rabbitmq::stream::Stream;
use rabbitmq::types::{Exchange, Queue};

/// Declare the given queues to the given `Channel`.
pub fn declare_queues<Q>(
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
                                &b.exchange(),
                                &b.routing_key(),
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
pub fn declare_exchanges<E>(
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
                                &b.exchange(),
                                &exchange.name(),
                                &b.routing_key(),
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

pub fn connect(
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

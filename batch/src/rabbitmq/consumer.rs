use std::io;
use std::fmt;
use std::result::Result as StdResult;

use futures::{self, future, Async, Future, Poll};
use lapin::channel::{BasicConsumeOptions, Channel};
use lapin::client::Client;
use lapin_async::queue::Message;
use lapin_async::types::FieldTable;
use tokio_core::reactor::Handle;

use error::{Error, ErrorKind};
use rabbitmq::common::{connect, declare_exchanges, declare_queues};
use rabbitmq::delivery::Delivery;
use rabbitmq::stream::Stream;
use rabbitmq::types::{Exchange, Queue};

/// A `Consumer` of incoming jobs.
///
/// The type of the stream is a tuple containing a `u64` which is a unique ID for the
/// job used when `ack`'ing or `reject`'ing it, and a `Job` instance.
pub struct Consumer {
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
            })
            .and_then(move |channel| {
                let channel_ = channel.clone();
                declare_exchanges(exchanges, channel_)
                    .map_err(|e| ErrorKind::Rabbitmq(e).into())
                    .map(|_| channel)
            })
            .and_then(move |channel| {
                let channel_ = channel.clone();
                declare_queues(queues_, channel_)
                    .map_err(|e| ErrorKind::Rabbitmq(e).into())
                    .map(|_| channel)
            })
            .and_then(|channel| {
                let consumer_channel = channel.clone();
                future::join_all(queues.into_iter().map(move |queue| {
                    consumer_channel
                        .basic_consume(
                            &queue.name(),
                            &format!("batch-rs-consumer-{}", queue.name()),
                            &BasicConsumeOptions::default(),
                            &FieldTable::new(),
                        )
                        .map_err(|e| ErrorKind::Rabbitmq(e).into())
                })).join(future::ok(channel))
            })
            .map(move |(mut consumers, channel)| {
                let initial: Box<
                    futures::Stream<Item = Message, Error = io::Error> + Send,
                > = Box::new(consumers.pop().unwrap());
                let stream = consumers.into_iter().fold(initial, |acc, consumer| {
                    Box::new(futures::Stream::select(acc, consumer))
                });
                Consumer { channel, stream }
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

use std::fmt;

use batch_core as batch;
use failure::Error;
use futures::sync::mpsc;
use futures::{task, Async, Future, Poll, Stream};
use lapin::channel;
use lapin::message;

use delivery::{Completion, Delivery};
use stream;

pub struct Consumer {
    inner: Box<Stream<Item = message::Delivery, Error = Error> + Send>,
    rx: Box<Future<Item = (), Error = ()> + Send>,
    tx: mpsc::Sender<Completion>,
}

impl Consumer {
    pub(crate) fn new(
        channel: channel::Channel<stream::Stream>,
        inner: Box<Stream<Item = message::Delivery, Error = Error> + Send + 'static>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(512);
        let rx = rx.for_each(
            move |completion| -> Box<Future<Item = (), Error = ()> + Send> {
                match completion {
                    Completion::Acknowledge(delivery_tag) => {
                        let task = channel.basic_ack(delivery_tag, false).map_err(|e| {
                            error!("An error occured while acknowledging a delivery: {}", e)
                        });
                        Box::new(task)
                    }
                    Completion::Reject(delivery_tag) => {
                        // TODO: Improve requeue logic. Currently, we're infinitely requeuing jobs on failure. Ideally,
                        // we'd honor the number of retries specified in the delivery's properties. Also, there isn't any
                        // delay between each execution, whereas we should have an exponential backoff between each
                        // execution.
                        let task = channel.basic_reject(delivery_tag, true).map_err(|e| {
                            error!("An error occured while rejecting a delivery: {}", e)
                        });
                        Box::new(task)
                    }
                }
            },
        );
        let rx = Box::new(rx);
        Consumer { inner, rx, tx }
    }
}

impl fmt::Debug for Consumer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Consumer").finish()
    }
}

impl batch::Consumer for Consumer {
    type Delivery = Delivery;
}

impl Stream for Consumer {
    type Item = Delivery;

    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Err(_) = self.rx.poll() {
            error!("An error occured while polling for deliveries completions");
            bail!("An error occured while polling for deliveries completions");
        }
        let delivery = match try_ready!(self.inner.poll().map_err(Error::from)) {
            Some(delivery) => delivery,
            None => return Ok(Async::Ready(None)),
        };
        let delivery = match Delivery::new(delivery, self.tx.clone()) {
            Ok(delivery) => delivery,
            Err(e) => {
                error!("Couldn't handle incoming delivery: {}", e);
                task::current().notify();
                return Ok(Async::NotReady);
            }
        };
        Ok(Async::Ready(Some(delivery)))
    }
}

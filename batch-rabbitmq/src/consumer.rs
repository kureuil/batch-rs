use batch;
use failure::Error;
use futures::{task, Async, Poll, Stream};
use lapin::channel;
use lapin::message;
use std::fmt;

use crate::delivery::Delivery;
use crate::stream;

#[must_use = "streams do nothing unless polled"]
pub struct Consumer {
    inner: Box<Stream<Item = message::Delivery, Error = Error> + Send>,
    channel: channel::Channel<stream::Stream>,
}

impl Consumer {
    pub(crate) fn new(
        channel: channel::Channel<stream::Stream>,
        inner: Box<Stream<Item = message::Delivery, Error = Error> + Send + 'static>,
    ) -> Self {
        Consumer { inner, channel }
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
        let delivery = match futures::try_ready!(self.inner.poll()) {
            Some(delivery) => delivery,
            None => return Ok(Async::Ready(None)),
        };
        let delivery = match Delivery::new(delivery, self.channel.clone()) {
            Ok(delivery) => delivery,
            Err(e) => {
                log::error!("Couldn't handle incoming delivery: {}", e);
                task::current().notify();
                return Ok(Async::NotReady);
            }
        };
        Ok(Async::Ready(Some(delivery)))
    }
}

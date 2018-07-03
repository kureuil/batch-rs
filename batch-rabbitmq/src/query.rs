use batch_core as batch;
use failure::Error;
use futures::sync::mpsc::Sender;
use futures::{future, Future, Sink};

use dispatch::Dispatch;

#[derive(Debug)]
pub struct Query<J> {
    publisher: Sender<Dispatch>,
    exchange: String,
    job: J,
    properties: batch::Properties,
}

impl<J> Query<J>
where
    J: batch::Job,
{
    pub(crate) fn new(publisher: Sender<Dispatch>, exchange: String, job: J) -> Self {
        Query {
            publisher,
            exchange,
            job,
            properties: batch::Properties::new(J::NAME),
        }
    }
}

impl<J> batch::dsl::WithPriority for Query<J>
where
    J: batch::Job,
{
    fn priority(mut self, priority: batch::Priority) -> Self {
        self.properties.priority = priority;
        self
    }
}

impl<J> batch::dsl::Deliver for Query<J>
where
    J: batch::Job,
{
    type DeliverFuture = Box<Future<Item = (), Error = Error> + Send>;

    fn deliver(self) -> Self::DeliverFuture {
        let delivery = match Dispatch::new(self.exchange.clone(), self.job, self.properties) {
            Ok(delivery) => delivery,
            Err(e) => return Box::new(future::err(e)),
        };
        let task = self
            .publisher
            .send(delivery)
            .map(|_| ())
            .map_err(|e| Error::from(e));
        Box::new(task)
    }
}

use batch;
use failure::Error;
use futures::{future, Future};
use serde_json;
use std::collections::BTreeMap;
use std::fmt;

use {ConnectionBuilder, Exchange};

/// A builder for the [`Queue`] type.
///
/// You should not have to construct values of this type yourself, and should instead let
/// `batch_rabbitmq`'s procedural macros generate the necessary invocations.
pub struct Builder {
    name: String,
    exchange: Exchange,
    callbacks: BTreeMap<
        &'static str,
        fn(&[u8], &batch::Factory) -> Box<Future<Item = (), Error = Error> + Send>,
    >,
}

impl fmt::Debug for Builder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Builder")
            .field("name", &self.name)
            .field("exchange", &self.exchange)
            .field("callbacks", &self.callbacks.keys())
            .finish()
    }
}

impl Builder {
    /// Binds the given job type and its associated callbacks to this queue.
    pub fn bind<J>(mut self) -> Self
    where
        J: batch::Job,
    {
        let callback = |payload: &[u8], context: &batch::Factory| // TODO: try to remote type annotations
         -> Box<Future<Item = (), Error = Error> + Send> {
            let job: J = match serde_json::from_slice(payload) {
                Ok(job) => job,
                Err(e) => return Box::new(future::err(Error::from(e))),
            };
            Box::new(job.perform(context))
        };
        self.callbacks.insert(J::NAME, callback);
        self
    }

    /// Set the exchange for the [`Queue`] that will be created.
    pub fn exchange(mut self, exchange: Exchange) -> Self {
        self.exchange = exchange;
        self
    }

    /// Create the [`Queue`] from the builder fields.
    pub fn finish(self) -> Queue {
        Queue {
            name: self.name,
            exchange: self.exchange,
            callbacks: self.callbacks,
        }
    }
}

/// A RabbitMQ Queue.
///
/// You should not have to construct values of this type yourself, and should instead let
/// `batch_rabbitmq`'s procedural macros generate the necessary invocations.
#[derive(Clone)]
pub struct Queue {
    name: String,
    exchange: Exchange,
    callbacks: BTreeMap<
        &'static str,
        fn(&[u8], &batch::Factory) -> Box<Future<Item = (), Error = Error> + Send>,
    >,
}

impl fmt::Debug for Queue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Builder")
            .field("name", &self.name)
            .field("exchange", &self.exchange)
            .field("callbacks", &self.callbacks.keys())
            .finish()
    }
}

impl Queue {
    /// Create a [`Builder`] for a [`Queue`] named `name`.
    pub fn build(name: impl Into<String>) -> Builder {
        let name = name.into();
        Builder {
            name: name.clone(),
            exchange: Exchange::new(name),
            callbacks: BTreeMap::new(),
        }
    }

    /// The name of this queue.
    ///
    /// # Example
    ///
    /// ```
    /// use batch_rabbitmq::Queue;
    ///
    /// let queue = Queue::build("batch-queue").finish();
    /// assert!(queue.name() == "batch-queue");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The exchange associated to this queue.
    pub fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    /// The callbacks associated to this queue.
    pub fn callbacks(
        &self,
    ) -> impl Iterator<
        Item = (
            &'static str,
            fn(&[u8], &batch::Factory) -> Box<Future<Item = (), Error = Error> + Send>,
        ),
    > {
        self.callbacks.clone().into_iter() // TODO: remove clone
    }

    /// Register this queue to the given [`ConnectionBuilder`].
    pub fn register(self, conn: &mut ConnectionBuilder) {
        conn.queues.insert(self.name().into(), self);
    }
}

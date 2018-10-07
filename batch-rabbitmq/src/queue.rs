use batch;
use failure::Error;
use futures::{future, Future};
use serde_json;
use std::collections::BTreeMap;
use std::fmt;

use {ConnectionBuilder, Exchange};

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

    pub fn exchange(mut self, exchange: Exchange) -> Self {
        self.exchange = exchange;
        self
    }

    pub fn finish(self) -> Queue {
        Queue {
            name: self.name,
            exchange: self.exchange,
            callbacks: self.callbacks,
        }
    }
}

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
    pub fn build(name: String) -> Builder {
        Builder {
            name: name.clone(),
            exchange: Exchange::new(name),
            callbacks: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn exchange(&self) -> &Exchange {
        &self.exchange
    }

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

    pub fn register(self, conn: &mut ConnectionBuilder) {
        conn.queues.insert(self.name().into(), self);
    }
}

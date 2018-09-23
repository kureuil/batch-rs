use std::sync::Arc;

use batch;
use failure::Error;
use futures::sync::mpsc::Sender;
use futures::Future;

use dispatch::Dispatch;
use query::Query;

/// Allowed types for RabbitMQ's exchanges.
///
/// Built-in types are represented as dedicated enum variants, and you can specify other exchange types using the
/// `Kind::Custom` variant.
#[derive(Debug, Clone)]
pub enum Kind {
    Direct,
    Fanout,
    Topic,
    Headers,
    Custom(String),
}

impl Default for Kind {
    fn default() -> Kind {
        Kind::Direct
    }
}

impl AsRef<str> for Kind {
    fn as_ref(&self) -> &str {
        match *self {
            Kind::Direct => "direct",
            Kind::Fanout => "fanout",
            Kind::Topic => "topic",
            Kind::Headers => "headers",
            Kind::Custom(ref name) => name.as_ref(),
        }
    }
}

/// A builder for the `Exchange` type.
#[derive(Debug, Clone)]
pub struct Builder {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) exclusive: bool,
}

impl Builder {
    pub fn kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    pub fn exclusive(mut self, exclusive: bool) -> Self {
        self.exclusive = exclusive;
        self
    }

    pub fn declare(
        self,
        declarator: &mut impl batch::Declarator<Builder, Exchange>,
    ) -> impl Future<Item = Exchange, Error = Error> {
        declarator.declare(self)
    }
}

/// A RabbitMQ exchange.
#[derive(Debug, Clone)]
pub struct Exchange {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    publisher: Sender<Dispatch>,
    name: String,
    kind: Kind,
}

impl Exchange {
    pub fn builder(name: String) -> Builder {
        Builder {
            name,
            kind: Kind::Direct,
            exclusive: false,
        }
    }

    pub(crate) fn new(publisher: Sender<Dispatch>, name: String, kind: Kind) -> Exchange {
        Exchange {
            inner: Arc::new(Inner {
                publisher,
                name,
                kind,
            }),
        }
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn kind(&self) -> &Kind {
        &self.inner.kind
    }
}

impl<J> batch::dsl::With<J> for Exchange
where
    J: batch::Job,
{
    type Query = Query<J>;

    fn with(&self, job: J) -> Self::Query {
        Query::new(self.inner.publisher.clone(), self.inner.name.clone(), job)
    }
}

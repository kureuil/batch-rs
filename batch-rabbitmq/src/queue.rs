use std::collections::btree_map::IntoIter;
use std::collections::BTreeMap;
use std::fmt;

use batch_core as batch;
use failure::Error;
use futures::{future, Future};
use serde::{Deserialize, Deserializer};
use serde_json;

#[derive(Debug, Clone)]
pub(crate) struct Binding {
    pub(crate) exchange: String,
    pub(crate) routing_key: String,
}

pub struct Builder {
    pub(crate) name: String,
    pub(crate) bindings: Vec<Binding>,
    pub(crate) callbacks:
        BTreeMap<String, fn(&[u8], batch::Factory) -> Box<Future<Item = (), Error = Error> + Send>>,
}

impl fmt::Debug for Builder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Builder")
            .field("name", &self.name)
            .field("bindings", &self.bindings)
            .field("callbacks", &self.callbacks.keys())
            .finish()
    }
}

impl Builder {
    pub fn bind<E, J>(mut self) -> Self
    where
        E: batch::Declare,
        J: batch::Job,
    {
        let callback = |payload: &[u8],
                        context: batch::Factory|
         -> Box<Future<Item = (), Error = Error> + Send> {
            struct De<J: batch::Job>(J);

            impl<'de, J: batch::Job> Deserialize<'de> for De<J> {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let inner: J = J::deserialize(deserializer)?;
                    Ok(De(inner))
                }
            }

            let job: J = match serde_json::from_slice(payload) {
                Ok(De(job)) => job,
                Err(e) => return Box::new(future::err(Error::from(e))),
            };
            Box::new(job.perform(context))
        };
        self.bindings.push(Binding {
            exchange: E::NAME.into(),
            routing_key: J::NAME.into(),
        });
        self.callbacks.insert(J::NAME.into(), callback);
        self
    }

    pub fn declare(
        self,
        declarator: &mut impl batch::Declarator<Builder, Queue>,
    ) -> impl Future<Item = Queue, Error = Error> {
        declarator.declare(self)
    }

    pub(crate) fn build(self) -> Result<Queue, Error> {
        Ok(Queue {
            name: self.name,
            bindings: self.bindings,
            callbacks: self.callbacks,
        })
    }
}

pub struct Queue {
    pub(crate) name: String,
    pub(crate) bindings: Vec<Binding>,
    pub(crate) callbacks:
        BTreeMap<String, fn(&[u8], batch::Factory) -> Box<Future<Item = (), Error = Error> + Send>>,
}

impl fmt::Debug for Queue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Builder")
            .field("name", &self.name)
            .field("bindings", &self.bindings)
            .field("callbacks", &self.callbacks.keys())
            .finish()
    }
}

impl Queue {
    pub fn builder(name: String) -> Builder {
        Builder {
            name,
            bindings: vec![],
            callbacks: BTreeMap::new(),
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

impl batch::Callbacks for Queue {
    type Iterator =
        IntoIter<String, fn(&[u8], batch::Factory) -> Box<Future<Item = (), Error = Error> + Send>>;

    fn callbacks(&self) -> Self::Iterator {
        self.callbacks.clone().into_iter()
    }
}

use failure::Error;
use futures::{Future, IntoFuture};
use std::marker::PhantomData;

use dispatch::Dispatch;

use {Client, Job, Properties, Queue};

/// A fluent interface to configure jobs before dispatching them.
#[derive(Debug)]
pub struct Query<J, Q> {
    job: J,
    properties: Properties,
    _marker: PhantomData<Q>,
}

impl<J, Q> Query<J, Q>
where
    J: Job,
    Q: Queue,
{
    /// Create a new `Query` instance from a `Job`.
    pub fn new(job: J) -> Self {
        Query {
            job,
            properties: Properties::new(J::NAME),
            _marker: PhantomData,
        }
    }

    /// Dispatch the job using the given `Client`.
    pub fn dispatch(self, client: &mut impl Client) -> impl Future<Item = (), Error = Error> {
        let mut client = client.clone();
        Dispatch::new(Q::NAME.into(), self.job, self.properties)
            .into_future()
            .and_then(move |dispatch| {
                client
                    .send(dispatch)
                    .map(|_| ())
                    .map_err(|e| Error::from(e))
            })
    }
}

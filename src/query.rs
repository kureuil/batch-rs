use failure::Error;
use futures::{Async, Future, Poll};
use log::debug;
use std::marker::PhantomData;

use crate::dispatch::Dispatch;

use crate::{Client, Job, Properties, Queue};

/// A fluent interface to configure jobs before dispatching them.
///
/// A `Query` is generic on both the job that will be published and the queue it will be published
/// to. This makes it possible to annotate the queue with marker traits that communicates about
/// the features it supports on a per job basis (e.g: per job priorities, per job time-to-live).
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
    ///
    /// This function is typically called from a wrapper generated by your adapter, but remains
    /// accessible if you ever need to write generic code.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate batch;
    /// # extern crate failure;
    /// # extern crate futures;
    /// #
    /// # use batch::{Factory, Queue};
    /// # use failure::Error;
    /// # use futures::Future;
    /// #
    /// # struct ExampleQueue;
    /// #
    /// # impl Queue for ExampleQueue {
    /// #     const SOURCE: &'static str = "example-queue";
    /// #
    /// #     const DESTINATION: &'static str = "example-queue";
    /// #
    /// #     type CallbacksIterator = Box<Iterator<
    /// #         Item = (
    /// #             &'static str,
    /// #             fn(&[u8], &Factory) -> Box<dyn Future<Item = (), Error = Error> + Send>,
    /// #         ),
    /// #     >>;
    /// #
    /// #     fn callbacks() -> Self::CallbacksIterator {
    /// #         unimplemented!()
    /// #     }
    /// # }
    /// use batch::{job, Query};
    ///
    /// #[job(name = "batch-example.say-hello")]
    /// fn say_hello(name: String) {
    ///     println!("Hello {}", name);
    /// }
    ///
    /// let query = Query::<_, ExampleQueue>::new(say_hello("Ferris".into()));
    /// ```
    pub fn new(job: J) -> Self {
        Query {
            job,
            properties: Properties::new(J::NAME),
            _marker: PhantomData,
        }
    }

    /// Dispatch the job using the given `Client`.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate batch;
    /// # extern crate batch_stub;
    /// # extern crate failure;
    /// # extern crate futures;
    /// # extern crate tokio;
    /// #
    /// # use batch::{Factory, Queue};
    /// # use failure::Error;
    /// # use futures::Future;
    /// #
    /// # struct ExampleQueue;
    /// #
    /// # impl Queue for ExampleQueue {
    /// #     const SOURCE: &'static str = "example-queue";
    /// #
    /// #     const DESTINATION: &'static str = "example-queue";
    /// #
    /// #     type CallbacksIterator = Box<Iterator<
    /// #         Item = (
    /// #             &'static str,
    /// #             fn(&[u8], &Factory) -> Box<dyn Future<Item = (), Error = Error> + Send>,
    /// #         ),
    /// #     >>;
    /// #
    /// #     fn callbacks() -> Self::CallbacksIterator {
    /// #         unimplemented!()
    /// #     }
    /// # }
    /// #
    /// # fn make_client() -> impl batch::Client {
    /// #     ::batch_stub::Client::new()
    /// # }
    /// use batch::{job, Query};
    ///
    /// #[job(name = "batch-example.say-hello")]
    /// fn say_hello(name: String) {
    ///     println!("Hello {}", name);
    /// }
    ///
    /// let mut client = make_client();
    /// let f = Query::<_, ExampleQueue>::new(say_hello("Ferris".into()))
    ///     .dispatch(&mut client)
    ///     .map_err(|e| eprintln!("An error occured: {}", e));
    ///
    /// # if false {
    /// // We're probably already in a tokio context so we use `spawn` instead of `run`.
    /// tokio::spawn(f);
    /// # }
    /// ```
    pub fn dispatch<C>(self, client: &mut C) -> DispatchFuture<C, J>
    where
        C: Client,
    {
        debug!("dispatch; job={} queue={}", J::NAME, Q::DESTINATION);
        let client = client.clone();
        DispatchFuture {
            client,
            state: DispatchState::Raw(Q::DESTINATION, Some(self.job), Some(self.properties)),
        }
    }
}

/// The future returned when a message is dispatched to a message broker.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct DispatchFuture<C, J>
where
    C: Client,
{
    client: C,
    state: DispatchState<J, C::SendFuture>,
}

#[derive(Debug)]
enum DispatchState<J, F> {
    Raw(&'static str, Option<J>, Option<Properties>),
    Polling(F),
}

impl<C, J> Future for DispatchFuture<C, J>
where
    C: Client,
    J: Job,
{
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.state {
            DispatchState::Raw(destination, ref mut job, ref mut props) => {
                let dispatch = Dispatch::new(
                    destination.into(),
                    job.take().unwrap(),
                    props.take().unwrap(),
                )?;
                self.state = DispatchState::Polling(self.client.send(dispatch));
                self.poll()
            }
            DispatchState::Polling(ref mut f) => {
                let _ = futures::try_ready!(f.poll());
                Ok(Async::Ready(()))
            }
        }
    }
}

use batch;
use failure::Error;
use futures::future;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod sealed {
    use batch;
    use failure::Error;
    use futures::future;
    use serde::{Deserialize, Serialize};

    /// Stub job used to trick the type system in `Connection::declare`.
    #[derive(Debug, Deserialize, Serialize)]
    pub struct StubJob;

    impl batch::Job for StubJob {
        const NAME: &'static str = "";

        type PerformFuture = future::FutureResult<(), Error>;

        fn perform(self, _ctx: &batch::Factory) -> Self::PerformFuture {
            future::ok(())
        }
    }
}

use self::sealed::StubJob;

/// A stub implentation of a batch `Client`.
///
/// This client implementation is made to be used in tests, to check that
/// your code is correctly enqueuing the correct number of jobs in the right
/// queues. It is not meant to be used as a real `Client` implementation in
/// a production app.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    dispatches: HashMap<String, Vec<batch::Dispatch>>,
}

impl Client {
    /// Create a new stub client instance.
    ///
    /// # Example
    ///
    /// ```
    /// use batch_stub::Client;
    ///
    /// let client = Client::new();
    /// ```
    pub fn new() -> Self {
        let inner = Inner {
            dispatches: HashMap::new(),
        };
        Client {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Count the number of jobs in the given queue.
    pub fn count<Q>(&self, _ctor: impl Fn(StubJob) -> batch::Query<StubJob, Q>) -> usize
    where
        Q: batch::Queue,
    {
        let inner = self.inner.lock().unwrap();
        inner
            .dispatches
            .get(Q::NAME.into())
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

impl batch::Client for Client {
    type SendFuture = future::FutureResult<(), Error>;

    fn send(&mut self, dispatch: batch::Dispatch) -> Self::SendFuture {
        let mut inner = self.inner.lock().unwrap();
        inner
            .dispatches
            .entry(dispatch.destination().into())
            .or_insert_with(Vec::new)
            .push(dispatch);
        future::ok(())
    }
}

//! Batch Worker.

use failure::{self, Error};
use futures::{self, future, Future, Poll, Stream};
use log::{debug, error, warn};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::io::{self, Read};
use std::process;
use std::result::Result;
use std::sync::mpsc;
use tokio_executor;
use wait_timeout::ChildExt;

use {Client, Delivery, Factory, Query, Queue};

mod sealed {
    use failure::Error;
    use futures::future;
    use serde::{Deserialize, Serialize};
    use {Factory, Job};

    /// Stub job used to trick the type system in `Connection::declare`.
    #[derive(Debug, Deserialize, Serialize)]
    pub struct StubJob;

    impl Job for StubJob {
        const NAME: &'static str = "";

        type PerformFuture = future::FutureResult<(), Error>;

        fn perform(self, _ctx: &Factory) -> Self::PerformFuture {
            future::ok(())
        }
    }
}

use self::sealed::StubJob;

/// A worker executes jobs fetched by consuming from a client.
///
/// The worker is responsible for polling the broker for jobs, deserializing them and execute
/// them. It should never ever crash and sould be resilient to panic-friendly job handlers. Its
/// `Broker` implementation is completely customizable by the user.
///
/// The most important thing to know about the worker is that it favours safety over performance.
/// For each incoming job, it will spawn a new process whose only goal is to perform the job.
/// Even if this is slower than just executing the function in a threadpool, it allows much more
/// control: timeouts wouldn't even be possible if we were running the jobs in-process. It also
/// protects against unpredictable crashes.
pub struct Worker<C> {
    client: C,
    queues: HashSet<String>,
    factory: Factory,
    callbacks:
        HashMap<String, fn(&[u8], &::Factory) -> Box<dyn Future<Item = (), Error = Error> + Send>>,
}

impl<C> fmt::Debug for Worker<C>
where
    C: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Worker")
            .field("client", &self.client)
            .field("queues", &self.queues)
            .field("callbacks", &self.callbacks.keys())
            .finish()
    }
}

impl<C> Worker<C>
where
    C: Client + Send + 'static,
{
    /// Create a new `Worker` from a `Client` implementation.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate batch;
    /// # extern crate batch_stub;
    /// #
    /// # use batch::Client;
    /// #
    /// # fn make_client() -> impl Client {
    /// #     ::batch_stub::Client::new()
    /// # }
    /// use batch::Worker;
    ///
    /// let client = make_client();
    /// let worker = Worker::new(client);
    /// ```
    pub fn new(client: C) -> Self {
        Worker {
            client,
            factory: Factory::new(),
            queues: HashSet::new(),
            callbacks: HashMap::new(),
        }
    }

    /// Provide a constructor for a given type.
    ///
    /// See [`Factory::provide`].
    pub fn provide<F, T>(mut self, init: F) -> Self
    where
        T: 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        self.factory.provide(init);
        self
    }

    /// Instruct the worker to consume jobs from the given queue.
    ///
    /// Note how the function takes a function returning a `Query` as parameter: you're not
    /// supposed to write this function yourself, it should be provided by your Batch adapter.
    ///
    /// # Panics
    ///
    /// If the given provides a callback for a job already registered, and the callbacks don't
    /// point to the same function, this method will panic.
    pub fn queue<Q>(mut self, _ctor: impl Fn(StubJob) -> Query<StubJob, Q>) -> Self
    where
        Q: Queue,
    {
        self.queues.insert(Q::NAME.into());
        for (job, callback) in Q::callbacks() {
            if let Some(previous) = self.callbacks.insert(job.into(), callback) {
                if previous as fn(_, _) -> _ != callback as fn(_, _) -> _ {
                    panic!(
                        "Two different callbacks were registered for the `{}` job.",
                        job
                    );
                }
            }
        }
        self
    }

    /// Consume deliveries from all of the declared resources.
    pub fn work(self) -> Work {
        if let Ok(job) = env::var("BATCHRS_WORKER_IS_EXECUTOR") {
            let (tx, rx) = mpsc::channel::<Result<(), Error>>();
            let tx2 = tx.clone();
            let f = self
                .execute(job)
                .map(move |_| tx.send(Ok(())).unwrap())
                .map_err(move |e| tx2.send(Err(e)).unwrap());
            tokio_executor::spawn(f);
            rx.recv().unwrap().unwrap();
            process::exit(0);
        }
        Work(Box::new(self.supervise()))
    }

    fn supervise(mut self) -> impl Future<Item = (), Error = Error> + Send {
        self.client
            .to_consumer(self.queues.clone().into_iter())
            .and_then(move |consumer| {
                consumer.for_each(move |delivery| {
                    debug!("delivery; job_id={}", delivery.properties().id);
                    // TODO: use tokio_threadpool::blocking instead of spawn a task for each execution?
                    let task = futures::lazy(
                        move || -> Box<dyn Future<Item = (), Error = Error> + Send> {
                            match spawn(&delivery) {
                                Err(e) => {
                                    error!("spawn: {}; job_id={}", e, delivery.properties().id);
                                    Box::new(delivery.reject())
                                }
                                Ok(ExecutionStatus::Failed(f)) => {
                                    warn!(
                                        "execution; status={:?} job_id={}",
                                        ExecutionStatus::Failed(f),
                                        delivery.properties().id
                                    );
                                    Box::new(delivery.reject())
                                }
                                Ok(ExecutionStatus::Success) => {
                                    debug!(
                                        "execution; status={:?} job_id={}",
                                        ExecutionStatus::Success,
                                        delivery.properties().id
                                    );
                                    Box::new(delivery.ack())
                                }
                            }
                        },
                    ).map_err(|e| {
                        error!("An error occured while informing the broker of the execution status: {}", e)
                    });
                    tokio_executor::spawn(task);
                    Ok(())
                })
            })
            .map(|_| ())
    }

    fn execute(self, job: String) -> impl Future<Item = (), Error = Error> + Send {
        let mut input = vec![];
        match io::stdin().read_to_end(&mut input).map_err(Error::from) {
            Ok(_) => (),
            Err(e) => {
                return Box::new(future::err(e)) as Box<Future<Item = (), Error = Error> + Send>
            }
        };
        let handler = match self.callbacks.get(&job) {
            Some(handler) => handler,
            None => {
                return Box::new(future::err(failure::err_msg(format!(
                    "No handler registered for {}",
                    job
                )))) as Box<Future<Item = (), Error = Error> + Send>
            }
        };
        Box::new((*handler)(&input, &self.factory)) as Box<Future<Item = (), Error = Error> + Send>
    }
}

/// The future returned when calling `Worker::work`.
#[must_use = "futures do nothing unless polled"]
pub struct Work(Box<Future<Item = (), Error = Error> + Send>);

impl fmt::Debug for Work {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Work").finish()
    }
}

impl Future for Work {
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

#[derive(Debug)]
enum ExecutionStatus {
    Success,
    Failed(ExecutionFailure),
}

#[derive(Debug)]
enum ExecutionFailure {
    Timeout,
    Crash,
    Error,
}

fn spawn(delivery: &impl Delivery) -> Result<ExecutionStatus, Error> {
    use std::io::Write;

    let current_exe = env::current_exe()?;
    let mut child = process::Command::new(&current_exe)
        .env("BATCHRS_WORKER_IS_EXECUTOR", &delivery.properties().task)
        .stdin(process::Stdio::piped())
        .spawn()?;
    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(delivery.payload())?;
        stdin.flush()?;
    }
    let (_, timeout) = delivery.properties().timelimit;
    if let Some(duration) = timeout {
        drop(child.stdin.take());
        if let Some(status) = child.wait_timeout(duration)? {
            if status.success() {
                Ok(ExecutionStatus::Success)
            } else if status.unix_signal().is_some() {
                Ok(ExecutionStatus::Failed(ExecutionFailure::Crash))
            } else {
                Ok(ExecutionStatus::Failed(ExecutionFailure::Error))
            }
        } else {
            child.kill()?;
            child.wait()?;
            Ok(ExecutionStatus::Failed(ExecutionFailure::Timeout))
        }
    } else {
        let status = child.wait()?;
        if status.success() {
            Ok(ExecutionStatus::Success)
        } else if status.code().is_some() {
            Ok(ExecutionStatus::Failed(ExecutionFailure::Error))
        } else {
            Ok(ExecutionStatus::Failed(ExecutionFailure::Crash))
        }
    }
}

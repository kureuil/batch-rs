//! Batch Worker.
//!
//! The worker is responsible for polling the broker for tasks, deserializing them an execute
//! them. It should never ever crash and sould be resilient to panic-friendly task handlers. Its
//! `Broker` implementation is completely customizable by the user.
//!
//! # Trade-offs
//!
//! The most important thing to know about the worker is that it favours safety over performance.
//! For each incoming job, it will spawn a new process whose only goal is to perform the task.
//! Even if this is slower than just executing the function in a threadpool, it allows much more
//! control: timeouts wouldn't even be possible if we were running the tasks in-process. It also
//! protects against unpredictable crashes

use std::collections::HashMap;
use std::env;
use std::fmt;
use std::io;
use std::process;
use std::result::Result as StdResult;
use std::sync::Arc;
use std::time::Duration;

use futures::{future, Future, IntoFuture, Stream};
use lapin::channel::{BasicProperties, BasicPublishOptions};
use tokio_core::reactor::{Core, Handle};
use wait_timeout::ChildExt;

use de;
use error::{self, Result};
use job::{Failure as JobFailure, Status as JobStatus};
use rabbitmq::{self, Exchange, ExchangeBuilder, Queue, QueueBuilder};
use ser;
use task::{Perform, Task};

/// Type of task handlers stored in `Worker`.
type WorkerFn<Ctx> = Fn(&[u8], Ctx) -> Result<()>;

/// A builder to ease the construction of `Worker` instances.
pub struct WorkerBuilder<Ctx> {
    connection_url: String,
    context: Ctx,
    exchanges: Vec<Exchange>,
    handle: Option<Handle>,
    handlers: HashMap<&'static str, Box<WorkerFn<Ctx>>>,
    retries: HashMap<&'static str, u32>,
    queues: Vec<Queue>,
}

impl<Ctx> fmt::Debug for WorkerBuilder<Ctx>
where
    Ctx: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(
            f,
            "WorkerBuilder {{ connection_url: {:?} context: {:?} exchanges: {:?} retries: {:?} queues: {:?} }}",
            self.connection_url, self.context, self.exchanges, self.retries, self.queues
        )
    }
}

impl<Ctx> WorkerBuilder<Ctx> {
    /// Create a new `WorkerBuilder` instance, using the mandatory context.
    ///
    /// The type of the given context is then used to typecheck the tasks registered on
    /// this builder.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::WorkerBuilder;
    ///
    /// let builder = WorkerBuilder::new(());
    /// ```
    pub fn new(context: Ctx) -> Self {
        WorkerBuilder {
            context,
            connection_url: "amqp://localhost/%2f".into(),
            exchanges: Vec::new(),
            queues: Vec::new(),
            handle: None,
            handlers: HashMap::new(),
            retries: HashMap::new(),
        }
    }

    /// Set the URL used to connect to `RabbitMQ`.
    ///
    /// The URL must be a valid AMQP connection URL (ex: `amqp://localhost/%2f`) using either the
    /// `amqp` protocol or the `amqps` protocol.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::WorkerBuilder;
    ///
    /// let builder = WorkerBuilder::new(())
    ///     .connection_url("amqp://guest:guest@localhost:5672/%2f");
    /// ```
    pub fn connection_url(mut self, url: &str) -> Self {
        self.connection_url = url.into();
        self
    }

    /// Add exchanges to be declared when connecting to `RabbitMQ`.
    ///
    /// See `exchange` documentation.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::{exchange, WorkerBuilder};
    ///
    /// let exchanges = vec![
    ///     exchange("batch.example"),
    /// ];
    /// let builder = WorkerBuilder::new(())
    ///     .exchanges(exchanges);
    /// ```
    pub fn exchanges<EIter>(mut self, exchanges: EIter) -> Self
    where
        EIter: IntoIterator<Item = ExchangeBuilder>,
    {
        self.exchanges
            .extend(exchanges.into_iter().map(|e| e.build()));
        self
    }

    /// Add queues to be declared when connecting to `RabbitMQ`.
    ///
    /// See `queue` documentation.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::{queue, WorkerBuilder};
    ///
    /// let queues = vec![
    ///     queue("hello-world").bind("batch.example", "hello-world"),
    /// ];
    /// let builder = WorkerBuilder::new(())
    ///     .queues(queues);
    /// ```
    pub fn queues<QIter>(mut self, queues: QIter) -> Self
    where
        QIter: IntoIterator<Item = QueueBuilder>,
    {
        self.queues.extend(queues.into_iter().map(|q| q.build()));
        self
    }

    /// Set the `Handle` to the Tokio reactor that should be used by the `Worker`.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate batch;
    /// # extern crate failure;
    /// # extern crate tokio_core;
    /// #
    /// use batch::WorkerBuilder;
    /// # use failure::Error;
    /// use tokio_core::reactor::Core;
    ///
    /// # fn main() {
    /// #     example().unwrap();
    /// # }
    /// #
    /// # fn example() -> Result<(), Error> {
    /// let core = Core::new()?;
    /// let handle = core.handle();
    /// let builder = WorkerBuilder::new(())
    ///     .handle(handle);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn handle(mut self, handle: Handle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Register a new `Task` to be handled by the `Worker`.
    ///
    /// The type of the `Task`'s `Context` must be the same as the `Worker`'s.
    ///
    /// # Example
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate batch;
    /// # extern crate serde;
    /// # #[macro_use]
    /// # extern crate serde_derive;
    /// #
    /// use batch::{Perform, WorkerBuilder};
    ///
    /// #[derive(Serialize, Deserialize, Task)]
    /// #[task_routing_key = "hello-world"]
    /// struct SayHello {
    ///     to: String,
    /// }
    ///
    /// impl Perform for SayHello {
    ///     type Context = ();
    ///
    ///     fn perform(&self, _ctx: Self::Context) {
    ///         println!("Hello {}", self.to);
    ///     }
    /// }
    ///
    /// # fn main() {
    /// let builder = WorkerBuilder::new(())
    ///     .task::<SayHello>();
    /// # }
    /// ```
    pub fn task<T>(mut self) -> Self
    where
        T: Task + Perform<Context = Ctx>,
    {
        self.handlers.insert(
            T::name(),
            Box::new(|data, ctx| -> Result<()> {
                let task: T = de::from_slice(data).map_err(error::ErrorKind::Deserialization)?;
                Perform::perform(&task, ctx);
                Ok(())
            }),
        );
        self.retries.insert(T::name(), T::retries());
        self
    }

    /// Create a new `Worker` instance from this builder data.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::WorkerBuilder;
    ///
    /// let builder = WorkerBuilder::new(())
    ///     .build();
    /// ```
    pub fn build(self) -> Result<Worker<Ctx>> {
        if self.handle.is_none() {
            Err(error::ErrorKind::NoHandle)?;
        }
        Ok(Worker {
            connection_url: self.connection_url,
            context: self.context,
            handle: self.handle.unwrap(),
            handlers: self.handlers,
            exchanges: self.exchanges,
            retries: self.retries,
            queues: self.queues,
        })
    }
}

/// Long-running worker polling tasks from the given `Broker`.
pub struct Worker<Ctx> {
    connection_url: String,
    context: Ctx,
    handle: Handle,
    handlers: HashMap<&'static str, Box<WorkerFn<Ctx>>>,
    retries: HashMap<&'static str, u32>,
    exchanges: Vec<Exchange>,
    queues: Vec<Queue>,
}

impl<Ctx> fmt::Debug for Worker<Ctx>
where
    Ctx: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(
            f,
            "Worker {{ connection_url: {:?} context: {:?} queues: {:?} retries: {:?} }}",
            self.connection_url, self.context, self.queues, self.retries
        )
    }
}

impl<Ctx> Worker<Ctx> {
    /// Runs the worker, polling tasks from the broker and executing them.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate batch;
    /// # extern crate failure;
    /// extern crate tokio_core;
    ///
    /// use batch::WorkerBuilder;
    /// # use failure::Error;
    /// use tokio_core::reactor::Core;
    ///
    /// fn main() {
    /// #   example().unwrap();
    /// # }
    /// #
    /// # fn example() -> Result<(), Error> {
    ///     let mut core = Core::new()?;
    ///     let handle = core.handle();
    ///     let worker = WorkerBuilder::new(())
    ///         .handle(handle)
    ///         .build()?;
    ///     let task = worker.run();
    ///
    /// # if false {
    ///     core.run(task)?;
    /// # }
    /// # Ok(())
    /// }
    /// ```
    pub fn run(self) -> Box<Future<Item = (), Error = error::Error>> {
        match env::var("BATCHRS_WORKER_IS_EXECUTOR") {
            Ok(_) => Box::new(self.execute().into_future()),
            Err(_) => self.supervise(),
        }
    }

    fn supervise(self) -> Box<Future<Item = (), Error = error::Error>> {
        let handle = self.handle;
        let connection_url = self.connection_url;
        let queues = self.queues;
        let exchanges = self.exchanges;
        let retries = self.retries;
        let task = rabbitmq::Consumer::new_with_handle(
            &connection_url,
            exchanges.clone(),
            queues.clone(),
            handle.clone(),
        ).join(rabbitmq::Publisher::new_with_handle(
            &connection_url,
            exchanges,
            queues,
            handle.clone(),
        ))
            .and_then(|(consumer, publisher)| {
                let publisher = Arc::new(publisher);
                let retries = Arc::new(retries);
                future::loop_fn(consumer.into_future(), move |f| {
                    let publisher = Arc::clone(&publisher);
                    let retries = Arc::clone(&retries);
                    let handle = handle.clone();
                    f.and_then(move |(next, consumer)| {
                        let delivery = match next {
                            Some(delivery) => delivery,
                            None => return Ok(future::Loop::Break(())),
                        };
                        let max_retries = *retries.get(delivery.task()).unwrap_or(&0);
                        let task = match spawn(&delivery) {
                            Err(e) => {
                                error!(
                                    "[{}] Couldn't spawn child process: {}",
                                    delivery.task_id(),
                                    e
                                );
                                reject(&consumer, publisher, delivery, max_retries)
                            }
                            Ok(status) => match status {
                                JobStatus::Success => {
                                    debug!("[{}] Child execution succeeded", delivery.task_id());
                                    consumer.ack(delivery.tag())
                                }
                                JobStatus::Failed(_) => {
                                    debug!("[{}] Child execution failed", delivery.task_id());
                                    reject(&consumer, publisher, delivery, max_retries)
                                }
                                _ => unreachable!(),
                            },
                        };
                        let task = task.map_err(move |e| {
                            error!("An error occured: {}", e);
                        });
                        handle.spawn(task);
                        Ok(future::Loop::Continue(consumer.into_future()))
                    }).or_else(|(e, consumer)| {
                        use failure::Fail;

                        let cause = match e.kind().cause() {
                            Some(cause) => format!(" Cause: {}", cause),
                            None => "".into(),
                        };
                        error!("Couldn't receive message from consumer: {}.{}", e, cause);
                        Ok(future::Loop::Continue(consumer.into_future()))
                    })
                })
            });
        Box::new(task)
    }

    fn execute(self) -> Result<()> {
        let delivery: rabbitmq::Delivery =
            de::from_reader(io::stdin()).map_err(error::ErrorKind::Deserialization)?;
        if let Some(handler) = self.handlers.get(delivery.task()) {
            if let Err(e) = (*handler)(delivery.data(), self.context) {
                error!("Couldn't process job: {}", e);
            }
        } else {
            warn!("No handler registered for job: `{}'", delivery.task());
        }
        Ok(())
    }
}

fn reject(
    consumer: &rabbitmq::Consumer,
    broker: Arc<rabbitmq::Publisher>,
    mut delivery: rabbitmq::Delivery,
    max_retries: u32,
) -> Box<Future<Item = (), Error = error::Error>> {
    let task = consumer.reject(delivery.tag());
    if delivery.should_retry(max_retries) {
        debug!(
            "[{}] Retry job after failure: {:?}",
            delivery.task_id(),
            delivery
        );
        Box::new(task.and_then(move |_| {
            broker.send(
                delivery.exchange(),
                delivery.routing_key(),
                delivery.data(),
                &BasicPublishOptions::default(),
                delivery.properties().clone(),
            )
        }))
    } else {
        task
    }
}

fn spawn(delivery: &rabbitmq::Delivery) -> Result<JobStatus> {
    use std::io::Write;

    let current_exe = env::current_exe().map_err(error::ErrorKind::SubProcessManagement)?;
    let mut child = process::Command::new(&current_exe)
        .env("BATCHRS_WORKER_IS_EXECUTOR", "1")
        .stdin(process::Stdio::piped())
        .spawn()
        .map_err(error::ErrorKind::SubProcessManagement)?;
    let payload = ser::to_vec(&delivery).map_err(error::ErrorKind::Serialization)?;
    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin
            .write_all(&payload)
            .map_err(error::ErrorKind::SubProcessManagement)?;
        stdin
            .flush()
            .map_err(error::ErrorKind::SubProcessManagement)?;
    }
    let (_, timeout) = delivery.timeout();
    if let Some(duration) = timeout {
        drop(child.stdin.take());
        if let Some(status) = child
            .wait_timeout(duration)
            .map_err(error::ErrorKind::SubProcessManagement)?
        {
            if status.success() {
                Ok(JobStatus::Success)
            } else if status.unix_signal().is_some() {
                Ok(JobStatus::Failed(JobFailure::Crash))
            } else {
                Ok(JobStatus::Failed(JobFailure::Error))
            }
        } else {
            child
                .kill()
                .map_err(error::ErrorKind::SubProcessManagement)?;
            child
                .wait()
                .map_err(error::ErrorKind::SubProcessManagement)?;
            Ok(JobStatus::Failed(JobFailure::Timeout))
        }
    } else {
        let status = child
            .wait()
            .map_err(error::ErrorKind::SubProcessManagement)?;
        if status.success() {
            Ok(JobStatus::Success)
        } else if status.code().is_some() {
            Ok(JobStatus::Failed(JobFailure::Error))
        } else {
            Ok(JobStatus::Failed(JobFailure::Crash))
        }
    }
}

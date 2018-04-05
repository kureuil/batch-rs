//! A serialized `Task` annotated with metadata.

use std::fmt;
use std::result::Result as StdResult;
use std::time::Duration;

use futures::{Future, IntoFuture};
use lapin::channel::{BasicProperties, BasicPublishOptions};
use lapin::types::{AMQPValue, FieldTable};
use uuid::Uuid;

use client::Client;
use error::{self, Error, Result};
use rabbitmq::Exchange;
use ser;
use task::{Priority, Task};

/// A `Query` is responsible for publishing jobs to `RabbitMQ`.
pub struct Query<T>
where
    T: Task + 'static,
{
    task: T,
    exchange: String,
    routing_key: String,
    timeout: Option<Duration>,
    retries: u32,
    options: BasicPublishOptions,
    properties: BasicProperties,
}

impl<T> fmt::Debug for Query<T>
where
    T: Task + fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(
            f,
            "Query {{ task: {:?} exchange: {:?} routing_key: {:?} timeout: {:?} retries: {:?} options: {:?} properties: {:?} }}",
            self.task,
            self.exchange,
            self.routing_key,
            self.timeout,
            self.retries,
            self.options,
            self.properties
        )
    }
}

impl<T> Query<T>
where
    T: Task + 'static,
{
    /// Create a new `Query` from a `Task` instance.
    pub fn new(task: T) -> Self {
        let task_id = Uuid::new_v4().to_string();
        let mut headers = FieldTable::new();
        headers.insert("lang".to_string(), AMQPValue::LongString("rs".to_string()));
        headers.insert(
            "task".to_string(),
            AMQPValue::LongString(T::name().to_string()),
        );
        headers.insert("id".to_string(), AMQPValue::LongString(task_id.clone()));
        headers.insert("root_id".to_string(), AMQPValue::Void);
        headers.insert("parent_id".to_string(), AMQPValue::Void);
        headers.insert("group".to_string(), AMQPValue::Void);
        headers.insert(
            "timelimit".to_string(),
            AMQPValue::FieldArray(vec![
                AMQPValue::Void,
                T::timeout().map_or(AMQPValue::Void, |d| AMQPValue::Timestamp(d.as_secs())),
            ]),
        );
        let properties = BasicProperties {
            priority: Some(T::priority().to_u8()),
            content_type: Some("application/json".to_string()),
            content_encoding: Some("utf-8".to_string()),
            headers: Some(headers),
            correlation_id: Some(task_id),
            ..Default::default()
        };
        Query {
            task,
            exchange: T::exchange().to_string(),
            routing_key: T::routing_key().to_string(),
            timeout: T::timeout(),
            retries: T::retries(),
            options: BasicPublishOptions::default(),
            properties,
        }
    }

    /// Return a reference the properties of this message.
    pub fn properties(&self) -> &BasicProperties {
        &self.properties
    }

    /// Return a mutable reference the properties of this message.
    pub fn properties_mut(&mut self) -> &mut BasicProperties {
        &mut self.properties
    }

    /// Return a reference the options of this message.
    pub fn options(&self) -> &BasicPublishOptions {
        &self.options
    }

    /// Return a mutable reference the options of this message.
    pub fn options_mut(&mut self) -> &mut BasicPublishOptions {
        &mut self.options
    }

    /// Set the exchange this task will be published to.
    pub fn exchange(mut self, exchange: &str) -> Self {
        self.exchange = exchange.into();
        self
    }

    /// Set the routing key associated with this task.
    pub fn routing_key(mut self, routing_key: &str) -> Self {
        self.routing_key = routing_key.into();
        self
    }

    /// Set the timeout associated to this task's execution.
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the number of allowed retries for this task.
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Set the priority for this task.
    pub fn priority(mut self, priority: Priority) -> Self {
        {
            let properties = self.properties_mut();
            properties.priority = Some(priority.to_u8());
        }
        self
    }

    /// Send the job using the given client.
    pub fn send(self, client: &Client) -> Box<Future<Item = (), Error = Error>> {
        let client = client.clone();
        let task = ser::to_vec(&self.task)
            .map_err(error::ErrorKind::Serialization)
            .into_future()
            .map_err(|e| e.into())
            .and_then(move |serialized| {
                client.send(
                    &self.exchange,
                    &self.routing_key,
                    &serialized,
                    &self.options,
                    self.properties,
                )
            });
        Box::new(task)
    }
}

/// Shorthand to create a new `Query` instance from a `Task`.
pub fn job<T>(task: T) -> Query<T>
where
    T: Task + 'static,
{
    Query::new(task)
}

/// The different states a `Job` can be in.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Status {
    /// The job was created but it wasn't sent/received yet.
    Pending,
    /// The job was received by a worker that started executing it.
    Started,
    /// The job completed successfully.
    Success,
    /// The job didn't complete successfully, see attached `Failure` cause.
    Failed(Failure),
}

/// Stores the reason for a job failure.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Failure {
    /// The task handler returned an error.
    Error,
    /// The task didn't complete in time.
    Timeout,
    /// The task crashed (panic, segfault, etc.) while executing.
    Crash,
}

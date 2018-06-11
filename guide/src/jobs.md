# Jobs

A job is a unit of work which computation can be deferred using `batch`. You
declare a job by implementing the `Job` trait on a structure. Note that
implementing this trait requires you to also implement [`serde`]'s `Serialize`
and `Deserialize` traits, this is required in order to safely send your jobs
between processes.

The `Job` trait allows you to specify a default configuration for your job
(e.g: default timeout, default number of retries). Usually you will use
`batch`'s procedural macros and derive the trait:

```rust
#[macro_use]
extern crate batch;
extern crate serde;
#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize, Job)]
#[job_routing_key = "messaging"]
struct SayHello {
    to: String,
}
```

[`serde`]: https://serde.rs

## `job_routing_key` attribute

The only mandatory attribute is `job_routing_key`, which is used to transfer a
job from an *exchange* (where the `Client` publishes) to a *queue* (where the
`Worker` consumes messages from).

## `job_name` attribute

> **Default value**: The name of the type deriving `Job`

This value is used to register and identify jobs in the worker, mapping a
`job_name` to a deserializer. This attribute should be unique in your project.

## `job_exchange` attribute

> **Default value**: empty string (default RabbitMQ exchange)

This value is used when publishing a job to RabbitMQ. If you set a custom
exchange name, you must ensure that it is declared before using it (see
[`ClientBuilder::exchanges`]).

## `job_timeout` attribute

> **Default value**: 900 seconds (15 minutes)

This attribute gives the number of seconds allowed before a job execution is
considered failed. If the execution of a job takes longer that the given
timeout, it is stopped, marked as failed and if needed tried again.

## `job_retries` attribute

> **Default value**: 2

This attribute gives the number of times a job should be tried again in case of
failure. When a job is retried, it is pushed as a new job would be with the
exact same attributes except for the `job_retries` attribute that gets
decremented.

## `job_priority` attribute

> **Default value**: [`Priority::Normal`]

This attribute is used to mark some jobs as more or less important than other
and prioritize them for the consumer.

[`ClientBuilder::exchanges`]: https://docs.rs/batch/0.1/batch/struct.ClientBuilder.html#method.exchanges
[`Priority::Normal`]: https://docs.rs/batch/0.1/batch/enum.Priority.html

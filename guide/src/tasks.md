# Tasks

A task is a unit of work which computation can be deferred using `batch`. You declare a task by implementing the `Task` trait on a structure. Note that implementing this trait requires you to also implement [`serde`](https://serde.rs/)'s `Serialize` and `Deserialize` traits, this is required in order to safely send your tasks between processes.

The `Task` trait allows you to specify a default configuration for your task (e.g: default timeout, default number of retries). Usually you will use `batch`'s procedural macros and derive the trait:

```rust
#[macro_use]
extern crate batch;
extern crate serde;
#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize, Task)]
#[task_routing_key = "messaging"]
struct SayHello {
    to: String,
}
```

## `task_routing_key` attribute

The only mandatory attribute is `task_routing_key`, which is used to transfer a task from an *exchange* (where the `Client` publishes) to a *queue* (where the `Worker` consumes messages from).

## `task_name` attribute

> **Default value**: The name of the type deriving `Task`

This value is used to register and identify tasks in the worker, mapping a `task_name` to a deserializer. This attribute should be unique in your project.

## `task_exchange` attribute

> **Default value**: empty string (default RabbitMQ exchange)

This value is used when publishing a task to RabbitMQ. If you set a custom exchange name, you must ensure that it is declared before using it (see [`ClientBuilder::exchanges`](https://docs.rs/batch/0.1/batch/struct.ClientBuilder.html#method.exchanges)).

## `task_timeout` attribute

> **Default value**: 900 seconds (15 minutes)

This attribute gives the number of seconds allowed before a task execution is considered failed. If the execution of a task takes longer that the given timeout, it is stopped, marked as failed and if needed tried again.

## `task_retries` attribute

> **Default value**: 2

The attribute gives the number of times a task should be tried again in case of failure. When a task is retried, it is pushed as a new task would be with the exact same attributes except for the `task_retries` attribute that gets decremented.

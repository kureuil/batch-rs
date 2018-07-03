# Batch

A background job library written in Rust.

Batch allows you to defer jobs to worker processes, by sending messages to a broker. It is a type-safe library that favors safety over performance in order to minimize risk and avoid mistakes. It leverages the [`futures`] & [`tokio`] crates to provide asynchronous operations to the user.

Batch doesn't tie you to a particular message broker implementation: a bunch of adapters of bundled in the library but you are free to write your own to accomodate your requirements. As of today batch provides adapters for the following message brokers:

* [RabbitMQ]: [Go to the associated guide section](./rabbitmq/index.html)
* [Faktory]: [Go to the associated guide section](./faktory/index.html)
* [Amazon SQS]: [Go to the associated guide section](./sqs/index.html)

Examples are available on [GitHub][examples] or you can continue and read the Getting Started guide for the message broker of your choice.


[`futures`]: https://crates.io/crates/futures
[`tokio`]: https://crates.io/crates/tokio
[RabbitMQ]: https://www.rabbitmq.com/
[Faktory]: https://contribsys.com/faktory/
[Amazon SQS]: https://aws.amazon.com/sqs/
[examples]: https://github.com/kureuil/batch-rs/tree/master/batch/examples

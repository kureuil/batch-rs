# RabbitMQ

[RabbitMQ] is a popular message broker that implements the AMQP/0.9.1 protocol often used in enterprise settings thanks to its replication & clustering capabilities.

Batch provides an official adapter for RabbitMQ by enabling the `rabbitmq` feature in your `Cargo.toml`:

```toml
[dependencies]
batch = { version = "0.2", features = ["rabbitmq"] }
```

Batch only supports versions of RabbitMQ that are officially supported by Pivotal, which means RabbitMQ 3.7+. If you encounter an problem using batch with an older version of RabbitMQ we won't be able to fix it. You are free to submit a bug fix but we reserve ourselves the right to refuse it if we think it will induce too much work to maintain it.

[RabbitMQ]: https://www.rabbitmq.com

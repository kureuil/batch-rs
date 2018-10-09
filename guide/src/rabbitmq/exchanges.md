# Exchanges

In RabbitMQ parlance you don't publish messages to a queue but to an exchange. Batch abstracts away this behavior behind a unified `Queue` trait. When you declare your queue, an exchange with the same name is implicitly declared. But sometimes you want to have control about what example declared and how. To do that, you can use the `exchange` property of the `queues!` macro:

```rust
extern crate batch;
extern crate batch_rabbitmq;
extern crate tokio;

use batch_rabbitmq::queues;
use tokio::prelude::*;

queues! {
    Example {
        name = "batch.example",
        exchange = "batch.example-exchange",
    }
}

fn main() {
    let fut = batch_rabbitmq::Connection::build("amqp://guest:guest@localhost:5672/%2f")
        .declare(Example)
        .connect()
        .and_then(|client| {
#           drop(client);
            /* The `batch.example-exchange` exchange is now declared. */
            Ok(())
        })
        .map_err(|e| eprintln!("An error occured while declaring the queue: {}", e));

# if false {
    tokio::run(fut);
# }
}
```

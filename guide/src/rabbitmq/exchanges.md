# Exchanges

When you're using RabbitMQ, an exchange is the place where you will publish your messages. Before being able to use an exchange, you must first declare it:

```rust
extern crate batch;
extern crate tokio;

use batch::Declare;
use batch::rabbitmq::{self, exchanges};
use tokio::prelude::Future;

exchanges! {
    Example {
        name = "batch.example"
    }
}

fn main() {
    let fut = rabbitmq::Connection::open("amqp://guest:guest@localhost:5672/%2f")
        .and_then(|mut conn| Example::declare(&mut conn))
        .and_then(|transcoding| {
#           drop(transcoding);
            /* The `Transcoding` exchange is now declared. */
            Ok(())
        })
        .map_err(|e| eprintln!("An error occured while declaring the exchange: {}", e));
    tokio::run(fut);
}
```

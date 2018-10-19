# Getting started

The first thing you'll want to do once you've installed `batch` is connect to a message broker. Batch provides a few adapters for popular choices amongst message brokers, but you can also write your own adapter if you want to. In this guide we'll use the RabbitMQ adapter (don't forget to enable the `rabbitmq` feature when installing batch).

Let's begin by connection to a broker:

```rust
extern crate batch;
extern crate batch_rabbitmq;
extern crate tokio;

use tokio::prelude::*;

fn main() {
    let f = batch_rabbitmq::Connection::open("amqp://guest:guest@localhost:5672/%2f")
        .map(|conn| {
#           drop(conn);
            println!("We're connected to RabbitMQ!");
        })
        .map_err(|e| eprintln!("An error occured while connecting to RabbitMQ: {}", e));

# if false {
    tokio::run(f);
# }
}
```

Now that we've acquired a connection to our RabbitMQ server, we'll write our first [job]. There are two ways of defining a job with batch: the high-level one consists of writing a function and annotate it with the `job` attribute, the low-level one consists of declaring a structure and implementing `batch::Job` manually. For now we'll use the high-level way:

```rust
extern crate batch;
extern crate batch_rabbitmq;
extern crate tokio;

use batch::job;
use tokio::prelude::*;

#[job(name = "batch-example.say-hello")]
fn say_hello(name: String) {
    println!("Hello {}!", name);
}

fn main() {
    let f = batch_rabbitmq::Connection::open("amqp://guest:guest@localhost:5672/%2f")
        .map(|conn| {
#           drop(conn);
            println!("We're connected to RabbitMQ!");
        })
        .map_err(|e| eprintln!("An error occured while connecting to RabbitMQ: {}", e));

# if false {
    tokio::run(f);
# }
}
```

> **Note**: the `job` procedural macro will generate a structure that derives Serde's `Serialize` & `Deserialize`. That means that the arguments of your function must implement these traits. 

> **Note**: The string given to the `job` procedural macro as a parameter of the name of the job. You should strive for unique job names, ideally structured by domain (e.g: prefix all jobs related to media files compression by `"media-compress."`).

Now that we have our job, we want to send it to our RabbitMQ server. To do that we need to declare a queue, using the `queues!` macro:

```rust
extern crate batch;
extern crate batch_rabbitmq;
extern crate tokio;

use batch::job;
use batch_rabbitmq::queues;
use tokio::prelude::*;

queues! {
    Example {
        name = "batch-example.exchange",
        bindings = [
            say_hello,
        ],
    }
}

#[job(name = "batch-example.say-hello")]
fn say_hello(name: String) {
    println!("Hello {}!", name);
}

fn main() {
    let f = batch_rabbitmq::Connection::build("amqp://guest:guest@localhost:5672/%2f")
        .declare(Example)
        .connect()
        .and_then(|mut client| {
            let job = say_hello("Ferris".to_string());
            Example(job).dispatch(&mut client)
        })
        .map_err(|e| eprintln!("An error occured while connecting to RabbitMQ: {}", e));

# if false {
    tokio::run(f);
# }
}
```

> Note how we're declaring the exchange by giving its name as a parameter and not by using the infamous turbofish syntax.

Now that our job has been published to our broker, we'll need to fetch it and assign a function to this job. To do this, we'll create a new program, a *[worker]*.

[worker]: ../worker/index.html
# Batch

A distributed task queue library written in Rust using RabbitMQ as a message broker.

This library allows you to send a task to a RabbitMQ broker, so that a worker will be able
to pull it and execute the associated handler. It leverages the `futures` and `tokio-core`
crates to provide asynchronous I/O operations.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
batch = "0.1"
```

> **Note**: Task serialization depends on [`serde`](https://serde.rs/), so you will have to add it to your project's dependencies as well.

Then add this to your crate root:

```
#[macro_use]
extern crate batch;
```

Examples are available on [GitHub](https://github.com/kureuil/batch-rs/tree/master/batch/examples) or you can continue and read the Getting Started guide.

## Getting started

The first thing you'll want to do once you've installed `batch` is connect to a RabbitMQ broker. We'll start by creating a `Client`:

```rust
extern crate batch;
extern crate tokio_core;

use batch::ClientBuilder;
use tokio_core::reactor::Core;

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = ClientBuilder::new()
        .connection_url("amqp://localhost/%2f")
        .handle(handle)
        .build();

    core.run(client).unwrap();
}
```

Now, that we're connected to our broker, we'll create our first task. A task is a work of unit that you want to asynchronously, becuse handling synchronously isn't possible or wouldn't be ideal (e.g sending a mail from a web API). The easiest of creating a task, is by declaring a structure, and derive `Task` on it:

```rust
#[macro_use]
extern crate batch;
#[macro_use]
extern crate serde;
extern crate tokio_core;

use batch::ClientBuilder;
use tokio_core::reactor::Core;

#[derive(Serialize, Deserialize, Task)]
#[task_routing_key = "hello-world"]
struct SayHello {
    to: String,
}

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = ClientBuilder::new()
        .connection_url("amqp://localhost/%2f")
        .handle(handle)
        .build();

    core.run(client).unwrap();
}
```

> **Note**: you can see that in addition to `Task`, we're also deriving `serde`'s `Serialize` & `Deserialize` traits. This is necessary in order to safely send task over the network.

> **Note**: When deriving `Task` we added the (mandatory) `task_routing_key` attribute, it is used by RabbitMQ to deliver your message to the right worker.

Now that we have our task, we can send it to our message broker:

```rust
#[macro_use]
extern crate batch;
extern crate futures;
#[macro_use]
extern crate serde;
extern crate tokio_core;

use batch::{job, ClientBuilder};
use futures::Future;
use tokio_core::reactor::Core;

#[derive(Serialize, Deserialize, Task)]
#[task_routing_key = "hello-world"]
struct SayHello {
    to: String,
}

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = ClientBuilder::new()
        .connection_url("amqp://localhost/%2f")
        .handle(handle)
        .build();

    let send = client.and_then(|client| {
        let task = SayHello {
            to: "Ferris".into()
        };

        job(task).send(&client)
    });

    core.run(send).unwrap();
}
```

Now that our task has been published to our broker, we'll need to fetch it and assign a function to this task. To do this, we'll create a new program, the *worker*:

```rust
#[macro_use]
extern crate batch;
#[macro_use]
extern crate serde;
extern crate tokio_core;

use batch::{queue, WorkerBuilder};
use tokio_core::reactor::Core;

#[derive(Serialize, Deserialize, Task)]
#[task_routing_key = "hello-world"]
struct SayHello {
    to: String,
}

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let queues = vec![queue("hello-world")];
    let worker = WorkerBuilder::new(())
        .connection_url("amqp://localhost/%2f")
        .queues(queues)
        .handle(handle)
        .build()
        .unwrap();

    core.run(worker.run()).unwrap();
}
```

In order to register our task on the worker, we'll need to make it executable by implementing the `Perform` trait:

```rust
#[macro_use]
extern crate batch;
#[macro_use]
extern crate serde;
extern crate tokio_core;

use batch::{queue, Perform, WorkerBuilder};
use tokio_core::reactor::Core;

#[derive(Serialize, Deserialize, Task)]
#[task_routing_key = "hello-world"]
struct SayHello {
    to: String,
}

impl Perform for SayHello {
    type Context = ();

    fn perform(&self, _ctx: Self::Context) {
        println!("Hello {}!", self.to);
    }
}

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let queues = vec![queue("hello-world")];
    let worker = WorkerBuilder::new(())
        .connection_url("amqp://localhost/%2f")
        .queues(queues)
        .handle(handle)
        .task::<SayHello>()
        .build()
        .unwrap();

    core.run(worker.run()).unwrap();
}
```

We can now run our *worker* program and see the `Hello Ferris!` message displayed in the terminal.

## Features

* `codegen` *(enabled by default)*: Automatically re-exports the procedurals macros of `batch-codegen` from the `batch` crate.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

# Worker

As explained in the *[Concepts]* chapter, a worker consumes & executes jobs delivered by a message broker. Batch comes with a worker implementation with semantics heavily inspired by the [Resque] project. Like Resque, it assumes chaos: eventually your jobs will crash, or get stuck computing a value, or will be unable to contact an external service, in any way the `Worker` process shouldn't be affected by the execution of the jobs it is responsible for and be as resilient as possible against failure.

Maintaining such guarantees means that this `Worker` implementation isn't the most performant one, it is however one of the safest if you're not sure that your jobs are infallible. For example, because this implementation supports job timeouts it has to execute the job in a new process which is considered an expensive operation compared to spawning a thread. Due to this behavior (spawning a new process for each job execution), we will refer to this implementation as a *"Forking Worker"*.

## Adding a worker to your project

The easiest way to integrate the forking worker is to create a new binary (e.g: `src/bin/worker.rs`). This makes sure that your main command-line interface will not conflict with the worker, and vice-versa. The `Worker` struct is built using a `Client` instance, this makes it possible to use the same `Worker` implementation with any message broker adapter. In this example, we will be using the RabbitMQ adapter:

```rust
extern crate batch;
extern crate batch_rabbitmq;
extern crate tokio;

use batch::job;
use batch_rabbitmq::queues;
use batch::Worker;
use tokio::prelude::*;

queues! {
    Example {
        name = "example",
        bindings = [
            say_hello
        ]
    }
}

#[job(name = "batch-example.say-hello")]
fn say_hello(name: String) {
    println!("Hello {}!", name);
}

fn main() {
    // First, we configure the connection to our message broker
    let f = batch_rabbitmq::Connection::build("amqp://guest:guest@localhost:5672/%2f")
        // We declare our queue & exchange against RabbitMQ
        .declare(Example)
        // We establish the connection
        .connect()
        // Then, we create our worker instance & register the queue we will consume from
        .map(|client| Worker::new(client).queue(Example))
        // And finally, we consume incoming jobs
        .and_then(|worker| worker.work())
        .map_err(|e| eprintln!("An error occured: {}", e));

# if false {
    tokio::run(f);
# }
}
```

## Worker-provided values

Some of your jobs will undoubtly have to depend on values that can't be serialized (e.g: a connection to a database or credentials for your third party services). On one hand, you can't really easily serialize them, on the other hand you don't want to re-instantiate every time you need them. Batch gives you a solution to this problem: you provide a callback returning an instance of a resource to your worker, and your worker will use them to fill out values marked as *"injected"* on your jobs.

```rust
extern crate batch;
extern crate batch_rabbitmq;
extern crate tokio;

use batch::job;
use batch_rabbitmq::queues;
use tokio::prelude::*;
#
# mod diesel {
#   pub struct PgConn;
# }

queues! {
    Maintenance {
        name = "maintenance",
        bindings = [
            count_active_users
        ]
    }
}

#[job(name = "batch-example.count-active-users", inject = [ db ])]
fn count_active_users(db: diesel::PgConn) {
    # drop(db);
    // ...
}

fn init_database_conn() -> diesel::PgConn {
    // ...
#     diesel::PgConn
}

fn main() {
    let f = batch_rabbitmq::Connection::build("amqp://guest:guest@localhost:5672/%2f")
        .declare(Maintenance)
        .connect()
        .map(|conn|
            batch::Worker::new(conn)
                .provide(init_database_conn)
                .queue(Maintenance)
        )
        .and_then(|worker| worker.work())
        .map_err(|e| eprintln!("An error occured while executing the worker: {}", e));

# if false {
    tokio::run(f);
# }
}
```

[Concepts]: ../concepts.html
[Resque]: https://github.com/resque/resque

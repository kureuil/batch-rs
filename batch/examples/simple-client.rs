#[macro_use]
extern crate batch;
extern crate env_logger;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio;

use batch::{exchange, job, queue, ClientBuilder};
use futures::{future, Future};

#[derive(Serialize, Deserialize, Task)]
#[task_name = "batch::SayHello"]
#[task_routing_key = "hello-world"]
struct SayHello {
    to: String,
}

fn main() {
    env_logger::init();
    println!("Starting RabbitMQ client example");
    let exchanges = vec![exchange("batch.example")];
    let queues = vec![queue("hello-world").bind("batch.example", "hello-world")];
    let client = ClientBuilder::new()
        .connection_url("amqp://localhost/%2f")
        .exchanges(exchanges)
        .queues(queues)
        .build();
    let send = client
        .and_then(|client| {
            let task = SayHello {
                to: "Ferris".into(),
            };
            job(task).exchange("batch.example").send(&client)
        })
        .map_err(|e| eprintln!("An error occured in the client: {}", e));
    tokio::run(future::lazy(|| {
        tokio::spawn(send);
        Ok(())
    }));
}

#[macro_use]
extern crate batch;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
extern crate tokio;

use batch::{exchange, job, queue, ClientBuilder};
use futures::{future, Future};

#[derive(Serialize, Deserialize, Job)]
#[job_name = "batch::SayHello"]
#[job_routing_key = "hello-world"]
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
            let to = "Ferris".to_string();

            job(SayHello { to }).exchange("batch.example").send(&client)
        })
        .map_err(|e| eprintln!("An error occured in the client: {}", e));
    tokio::run(future::lazy(|| {
        tokio::spawn(send);
        Ok(())
    }));
}

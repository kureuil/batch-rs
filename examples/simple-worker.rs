#[macro_use]
extern crate batch;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
extern crate tokio;

use batch::{exchange, queue, Perform, Worker};
use futures::Future;
use std::{thread, time};

#[derive(Serialize, Deserialize, Job)]
#[job_name = "batch::SayHello"]
#[job_routing_key = "hello-world"]
struct SayHello {
    to: String,
}

impl Perform for SayHello {
    type Context = ();

    fn perform(&self, _ctx: Self::Context) {
        println!("Hello {}", self.to);
        let second = time::Duration::from_secs(1);
        thread::sleep(second);
        println!("Goodbye {}", self.to);
    }
}

fn main() {
    env_logger::init();
    println!("Starting RabbitMQ worker example");
    let exchanges = vec![exchange("batch.example")];
    let queues = vec![queue("hello-world").bind("batch.example", "hello-world")];
    let worker = Worker::builder(())
        .connection_url("amqp://localhost/%2f")
        .exchanges(exchanges)
        .queues(queues)
        .job::<SayHello>()
        .build()
        .unwrap();
    tokio::run(
        worker
            .run()
            .map_err(|e| eprintln!("An error occured in the Worker: {}", e)),
    );
}

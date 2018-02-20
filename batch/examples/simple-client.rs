#[macro_use]
extern crate batch;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate serde;
extern crate tokio_core;

use batch::{exchange, job, ClientBuilder};
use futures::Future;
use tokio_core::reactor::Core;

#[derive(Serialize, Deserialize, Task)]
#[task_routing_key = "hello-world"]
struct SayHello {
    to: String,
}

fn main() {
    env_logger::init();
    println!("Starting RabbitMQ client example");
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let exchanges = vec![exchange("batch.example")];
    let client = ClientBuilder::new()
        .connection_url("amqp://localhost/%2f")
        .exchanges(exchanges)
        .handle(handle)
        .build();
    let send = client.and_then(|client| {
        let task = SayHello {
            to: "Ferris".into(),
        };
        job(task).exchange("batch.example").send(&client)
    });
    core.run(send).unwrap();
}

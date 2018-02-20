#[macro_use]
extern crate batch;
extern crate env_logger;
#[macro_use]
extern crate serde;
extern crate tokio_core;

use batch::{exchange, queue, Perform, WorkerBuilder};
use tokio_core::reactor::Core;

#[derive(Serialize, Deserialize, Task)]
#[task_routing_key = "hello-world"]
struct SayHello {
    to: String,
}

impl Perform for SayHello {
    type Context = ();

    fn perform(&self, _ctx: Self::Context) {
        println!("Hello {}", self.to);
    }
}

fn main() {
    env_logger::init();
    println!("Starting RabbitMQ worker example");
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let exchanges = vec![exchange("batch.example")];
    let queues = vec![queue("hello-world").bind("batch.example", "hello-world")];
    let worker = WorkerBuilder::new(())
        .connection_url("amqp://localhost/%2f")
        .exchanges(exchanges)
        .queues(queues)
        .handle(handle)
        .task::<SayHello>()
        .build()
        .unwrap();
    core.run(worker.run()).unwrap();
}

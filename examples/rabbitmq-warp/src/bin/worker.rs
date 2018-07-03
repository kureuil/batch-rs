extern crate batch;
extern crate batch_example_rabbitmq_warp as example;
extern crate env_logger;
extern crate failure;
extern crate tokio;

use batch::rabbitmq;
use tokio::prelude::Future;

use example::queues;

fn main() {
    env_logger::init();
    let task = rabbitmq::Connection::open("amqp://guest:guest@localhost:5672/%2f")
        .map(|connection| batch::Worker::new(connection))
        .and_then(|worker| worker.declare::<queues::Transcoding>())
        .and_then(|worker| worker.run())
        .map_err(|e| eprintln!("An error occured: {}", e));
    tokio::run(task);
}

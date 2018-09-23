extern crate batch;
extern crate batch_example_rabbitmq_warp as example;
extern crate batch_rabbitmq;
extern crate env_logger;
extern crate failure;
extern crate tokio;

use tokio::prelude::Future;

use example::queues;

fn main() {
    env_logger::init();
    let task = batch_rabbitmq::Connection::open("amqp://guest:guest@localhost:5672/%2f")
        .map(|connection| batch::Worker::new(connection))
        .and_then(|worker| worker.declare::<queues::Transcoding>())
        .and_then(|worker| worker.run())
        .map_err(|e| eprintln!("An error occured: {}", e));
    tokio::run(task);
}

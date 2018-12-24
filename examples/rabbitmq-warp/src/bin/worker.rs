extern crate batch_example_rabbitmq_warp as example;

use tokio::prelude::Future;

use crate::example::queues;

fn main() {
    env_logger::init();
    let task = batch_rabbitmq::Connection::build("amqp://guest:guest@localhost:5672/%2f")
        .declare(queues::Transcoding)
        .connect()
        .map(|connection| batch::Worker::new(connection).queue(queues::Transcoding))
        .and_then(|worker| worker.work())
        .map_err(|e| eprintln!("An error occured: {}", e));
    tokio::run(task);
}

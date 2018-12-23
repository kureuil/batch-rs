extern crate batch_example_rabbitmq_warp as example;

use tokio::prelude::Future;

use crate::example::{endpoints, queues};

fn main() {
    let task = batch_rabbitmq::Connection::build("amqp://guest:guest@localhost/%2f")
        .declare(queues::Transcoding)
        .connect()
        .map_err(|e| eprintln!("An error occured: {}", e))
        .and_then(|conn| warp::serve(endpoints::endpoints(conn)).bind(([127, 0, 0, 1], 3030)));

    tokio::run(task);
}

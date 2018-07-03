extern crate batch;
extern crate batch_example_rabbitmq_simple as example;
extern crate env_logger;
extern crate failure;
extern crate futures;
extern crate tokio;

use batch::dsl::*;
use batch::{rabbitmq, Declare};
use futures::Future;

use example::{exchanges, jobs};

fn main() {
    env_logger::init();
    let task = rabbitmq::Connection::open("amqp://guest:guest@localhost:5672/%2f")
        .and_then(|mut conn| exchanges::Transcoding::declare(&mut conn))
        .and_then(|transcoding| {
            let job =
                jobs::convert_video_file("./westworld-2x06.mkv".into(), jobs::VideoFormat::Mpeg4);
            transcoding.with(job).deliver()
        })
        .map_err(|e| eprintln!("An error occured: {}", e));
    tokio::run(task);
}

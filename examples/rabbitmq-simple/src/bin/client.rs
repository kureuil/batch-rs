extern crate batch;
extern crate batch_example_rabbitmq_standalone as example;
extern crate batch_rabbitmq;
extern crate env_logger;
extern crate tokio;

use tokio::prelude::Future;

use example::{jobs, queues};

fn main() {
    env_logger::init();
    let task = batch_rabbitmq::Connection::build("amqp://guest:guest@localhost:5672/%2f")
        .declare(queues::Transcoding)
        .connect()
        .and_then(|mut client| {
            let filepath = "./westworld-2x06.mkv".into();
            let job = jobs::convert_video_file(filepath, jobs::VideoFormat::Mpeg4);
            queues::Transcoding(job).dispatch(&mut client)
        }).map_err(|e| eprintln!("An error occured: {}", e));
    tokio::run(task);
}

extern crate batch;
extern crate batch_example_rabbitmq_warp as example;
extern crate tokio;
extern crate warp;

use std::sync::Arc;

use batch::dsl::{Deliver, With};
use batch::{rabbitmq, Declare};
use warp::{Filter, Future};

use example::{exchanges, jobs};

fn main() {
    let task = rabbitmq::Connection::open("amqp://guest:guest@localhost/%2f")
        .and_then(|mut conn| exchanges::Transcoding::declare(&mut conn))
        .map_err(|e| println!("An error occured while declaring the exchange: {}", e))
        .and_then(|transcoding| {
            let transcoding = Arc::new(transcoding);
            let transcode = warp::get2().and(warp::path("transcode")).and_then(move || {
                let job = jobs::convert_video_file(
                    "./westworld-2x06.mkv".into(),
                    jobs::VideoFormat::Mpeg4,
                );
                transcoding
                    .with(job)
                    .deliver()
                    .map(|_| warp::reply())
                    .map_err(|_e| warp::reject::server_error())
            });

            warp::serve(transcode).bind(([127, 0, 0, 1], 3030))
        });

    tokio::run(task);
}

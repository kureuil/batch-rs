extern crate batch;
extern crate batch_rabbitmq;
extern crate env_logger;
extern crate log;
extern crate tokio;

use batch::job;
use batch_rabbitmq::queues;

#[job(name = "convert-video-file")]
fn convert_video_file(path: String) {
    println!("Converting {}...", path);
}

#[job(name = "say-hello")]
fn say_hello(name: String) {
    println!("Hello: {}", name);
}

queues! {
    Transcoding {
        name = "tests-consume.transcoding",
        bindings = [
            convert_video_file,
            say_hello,
        ],
    }
}

#[test]
fn consume_from_basic_queue() {
    use batch::{Client, Delivery, Job, Queue};
    use batch_rabbitmq::Connection;
    use log::info;
    use std::collections::VecDeque;
    use tokio::prelude::*;

    let _ = ::env_logger::try_init();
    let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
    let mut conn = {
        let f = Connection::build("amqp://guest:guest@localhost:5672/%2f")
            .declare(Transcoding)
            .connect();
        runtime.block_on(f).unwrap()
    };
    let consumer = {
        let f = conn.to_consumer(vec![Transcoding::SOURCE]);
        runtime.block_on(f).unwrap()
    };
    info!("Connected to RabbitMQ");
    {
        let job = convert_video_file("./westworld-2x06.mkv".into());
        let f = Transcoding(job).dispatch(&mut conn);
        let _ = runtime.block_on(f).unwrap();
    }
    info!("Published first message");
    {
        let job = say_hello("Ferris".into());
        let f = Transcoding(job).dispatch(&mut conn);
        let _ = runtime.block_on(f).unwrap();
    }
    info!("Published second message");
    info!("Published all messages");
    let expected = VecDeque::from(vec![convert_video_file::NAME, say_hello::NAME]);
    let mut consumer = consumer.into_future();
    info!("Iterating over consumer deliveries");
    for job in expected {
        info!("expecting: {:?}", job);
        let (delivery, next) = runtime.block_on(consumer).unwrap();
        assert_eq!(true, delivery.is_some());
        let delivery = delivery.unwrap();
        assert_eq!(delivery.properties().task, job);
        let _ = runtime.block_on(delivery.ack()).unwrap();
        consumer = next.into_future();
    }
    {
        info!("Dropping RabbitMQ connection in order to stop background task");
        drop(conn);
    }
    // FIXME: we're not checking whether deliveries have been ack'ed correctly.
    //        Calling basic.get on tests-consume.transcoding using lapin should be enough.
    runtime.run().unwrap();
}

mod common;
mod consumer;
mod delivery;
mod publisher;
mod stream;
mod types;

pub use self::consumer::Consumer;
pub use self::delivery::Delivery;
pub use self::publisher::Publisher;
pub use self::types::{exchange, queue, Exchange, ExchangeBuilder, Queue, QueueBuilder};

#[cfg(test)]
mod tests {
    use super::*;
    use task::Priority;

    #[test]
    fn priority_queue() {
        use std::collections::VecDeque;
        use futures::{future, Future, Stream};
        use lapin::channel::{BasicProperties, BasicPublishOptions};
        use lapin_async::types::{AMQPValue, FieldTable};
        use tokio_core::reactor::Core;

        ::env_logger::init();
        let mut core = Core::new().unwrap();
        let ex = "batch.tests.priorities";
        let rk = "prioritised-hello";
        let body = "{}";
        let jobs = vec![
            (("job-1", ex, rk, body.as_bytes()), Priority::Normal),
            (("job-2", ex, rk, body.as_bytes()), Priority::Critical),
            (("job-3", ex, rk, body.as_bytes()), Priority::Trivial),
            (("job-4", ex, rk, body.as_bytes()), Priority::High),
            (("job-5", ex, rk, body.as_bytes()), Priority::Low),
        ];
        let expected = VecDeque::from(vec!["job-2", "job-4", "job-1", "job-5", "job-3"]);

        let conn_url = "amqp://localhost/%2f";
        let exchanges = vec![exchange(ex).build()];
        let queues = vec![
            queue("tests.priorities")
                .enable_priorities()
                .bind(ex, rk)
                .build(),
        ];
        let handle = core.handle();
        let task = Publisher::new_with_handle(conn_url, exchanges.clone(), handle.clone())
            .and_then(|publisher| {
                let tasks = jobs.iter().map(move |&(ref job, ref priority)| {
                    let mut headers = FieldTable::new();
                    headers.insert("lang".to_string(), AMQPValue::LongString("rs".to_string()));
                    headers.insert("task".to_string(), AMQPValue::LongString(job.0.to_string()));
                    let properties = BasicProperties {
                        priority: Some(priority.to_u8()),
                        headers: Some(headers),
                        ..Default::default()
                    };
                    publisher.send(
                        job.1,
                        job.2,
                        job.3,
                        &BasicPublishOptions::default(),
                        properties,
                    )
                });
                future::join_all(tasks)
            })
            .and_then(move |_| Consumer::new_with_handle(conn_url, exchanges, queues, handle))
            .and_then(|consumer| {
                println!("Created consumer: {:?}", consumer);
                future::loop_fn(
                    (consumer.into_future(), expected.clone()),
                    |(f, mut order)| {
                        println!("Start iterating over consumer: {:?} // {:?}", f, order);
                        f.map_err(|(e, _)| e)
                            .and_then(move |(next, consumer)| {
                                println!("Got delivery: {:?}", next);
                                let head = order.pop_front().unwrap();
                                let tail = order;
                                let delivery = next.unwrap();
                                println!("Comparing: {:?} to {:?}", delivery.task(), head);
                                assert_eq!(delivery.task(), head);
                                consumer.ack(delivery.tag()).map(|_| (consumer, tail))
                            })
                            .and_then(|(consumer, order)| {
                                if order.is_empty() {
                                    Ok(future::Loop::Break(()))
                                } else {
                                    Ok(future::Loop::Continue((consumer.into_future(), order)))
                                }
                            })
                    },
                )
            });
        core.run(task).unwrap();
    }
}

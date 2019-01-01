use batch::Queue;
use redis::Commands;
use tokio::runtime::current_thread;

pub struct TestQueue {
    inner: String
}

pub fn TestQueue<J>(job: J) -> batch::Query<J, TestQueue>
where
    J: batch::Job
{
    batch::Query::new(job)
}

impl Queue for TestQueue {
    const SOURCE: &'static str = "test-queue";

    const DESTINATION: &'static str = "test-queue";

    type CallbacksIterator = std::vec::IntoIter<(&'static str, fn(&[u8], &batch::Factory) -> Box<dyn futures::Future<Item = (), Error = failure::Error> + Send>)>;

    fn callbacks() -> Self::CallbacksIterator {
        vec![].into_iter()
    }
}

fn fetch_redis_url() -> String {
    use std::env::{self, VarError};

    let provider = env::var("TEST_REDIS_PROVIDER").unwrap_or_else(|e| match e {
        VarError::NotPresent => "TEST_REDIS_URL".into(),
        VarError::NotUnicode(_) => panic!("The TEST_REDIS_PROVIDER environment variable is not unicode compliant"),
    });
    env::var(provider).unwrap_or_else(|e| match e {
        VarError::NotPresent => "redis://localhost:6379/0".into(),
        VarError::NotUnicode(_) => panic!("The TEST_REDIS_URL environment variable is not unicode compliant"),
    })
}

fn flush_redis_db(url: &str) {
    let client = redis::Client::open(url).unwrap();
    let conn = client.get_connection().unwrap();
    redis::cmd("FLUSHDB").execute(&conn);
}

#[test]
fn test_sending_job_to_redis_server() {
    let redis_url = fetch_redis_url();
    flush_redis_db(redis_url.as_ref());
    let mut runtime = current_thread::Runtime::new().unwrap();
    let fut = batch_redis::Connection::open(redis_url.as_ref());
    let mut conn = runtime
        .block_on(fut)
        .expect("Couldn't establish connection to Redis server");
    let job = batch_test::jobs::standard();
    let fut = TestQueue(job).dispatch(&mut conn);
    runtime.block_on(fut).expect("Couldn't dispatch job to Redis server");
    {
        let client = redis::Client::open(redis_url.as_ref()).unwrap();
        let conn = client.get_connection().unwrap();
        assert_eq!(conn.llen::<String, u64>(TestQueue::DESTINATION.into()).unwrap(), 1);
    }
}

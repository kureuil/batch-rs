extern crate batch_redis;
extern crate tokio;

use batch_redis::Connection;
use tokio::runtime::current_thread;

#[test]
fn test_connection_to_running_redis_server() {
	let mut runtime = current_thread::Runtime::new().unwrap();
	let fut = Connection::open("redis://localhost:6379/0");
	let _ = runtime.block_on(fut).unwrap();
}

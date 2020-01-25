//! Batch is a background job library.
//!
//! Batch allow a program (the *"Client"*) to defer a unit of work (the *"Job"*), until a background process (the
//! *"Worker"*) picks it up and executes it. This is very common in web development where you don't want to slow down
//! an HTTP request for a behaviour that doesn't has to be done synchronously (e.g: sending a mail when a user signs
//! up). Batch is compatible and should run correctly on Windows, macOS and Linux.
//!
//! Batch can work with any message broker as long as a `Client` can be implemented for it. The Batch project is
//! responsible for the development & maintenance of the following adapters:
//!
//! * [RabbitMQ](https://docs.rs/batch-rabbitmq/0.2)
//! * [Stub](https://docs.rs/batch-stub/0.2)
//!
//! Batch provides a worker implementation with sensible defaults that supports parallelism and job timeouts out of
//! the box, on all major platforms.
//!
//! # Example
//!
//! ```rust
//! const GREETINGS = batch::topic("greetings");
//!
//! #[batch::job(name = "say-hello")]
//! fn say_hello(name: String) {
//!     println!("Hello {}!", name);
//! }
//!
//! #[tokio::main]
//! async fn main() -> std::result::Result<(), Box<dyn Error>> {
//! #   if true { return Ok(()); }
//!     let connection = batch::rabbitmq::Connection::open("amqp://guest:guest@localhost/%2f").await?;
//!     connection.declare(&GREETINGS).await?;
//!     let client = batch::Client::new(connection).await?;
//!     say_hello(String::from("Ferris"))
//!         .on_topic(&GREETINGS)
//!         .dispatch(&mut client)
//!         .await?;
//!     Ok(())
//! } 
//! ```

#![doc(html_root_url = "https://docs.rs/batch/0.2.0")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod client;
mod delivery;
mod dispatch;
/// Not public API. This module is exempt from any semver guarantees.
#[doc(hidden)]
pub mod export;
mod factory;
mod job;
mod query;
mod queue;
mod worker;

pub use crate::client::{Client, Consumer};
pub use crate::delivery::Delivery;
pub use crate::dispatch::Dispatch;
pub use crate::factory::Factory;
pub use crate::job::{Job, Properties};
pub use crate::query::{DispatchFuture, Query};
pub use crate::queue::Queue;
pub use crate::worker::{Work, Worker};
#[cfg(feature = "codegen")]
pub use batch_codegen::job;

#[derive(Debug)]
pub(crate) struct QuickError(String);

impl QuickError {
	pub(crate) fn boxed(contents: impl std::fmt::Display) -> Box<dyn std::error::Error + Send> {
		Box::new(QuickError(contents.to_string()))
	}
}

impl std::fmt::Display for QuickError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::fmt::Display::fmt(&self.0, f)
	}
}

impl std::error::Error for QuickError {}

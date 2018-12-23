//! Batch is a background job library.
//!
//! Batch allow a program (the *"Client"*) to defer a unit of work (the *"Job"*), until a background process (the
//! *"Worker"*)  picks it up and executes it. This is very common in web development where you don't want to slow down
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
//! the box, on all supported platforms.
//!
//! # Example
//!
//! ```rust
//! extern crate batch;
//! extern crate batch_rabbitmq;
//! extern crate tokio;
//!
//! use batch::{job, Query};
//! use batch_rabbitmq::queues;
//! use tokio::prelude::*;
//!
//! queues! {
//!     Notifications {
//!         name = "notifications",
//!         bindings = [
//!             self::say_hello,
//!         ]
//!     }
//! }
//!
//! #[job(name = "batch-example.say-hello")]
//! fn say_hello(name: String) {
//!     println!("Hello {}!", name);
//! }
//!
//! fn main() {
//!     let fut = batch_rabbitmq::Connection::build("amqp://guest:guest@localhost/%2f")
//!         .declare(Notifications)
//!         .connect()
//!         .and_then(|mut client| {
//!             let job = say_hello(String::from("Ferris"));
//!             Notifications(job).dispatch(&mut client)
//!         });
//!
//! # if false {
//!     tokio::run(
//!         fut.map_err(|e| eprintln!("Couldn't publish message: {}", e))
//!     );
//! # }
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

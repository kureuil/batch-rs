//! Batch is a background job library.
//!
//! Batch allow a program (the *"Client"*) to defer a unit of work (the *"Job"*), until a background process (the
//! *"Worker"*)  picks it up and executes it. This is very common in web development where you don't want to slow down
//! an HTTP request for a behaviour that doesn't has to be done synchronously (e.g: sending a mail when a user signs
//! up).
//!
//! Batch provides you with adapters for popular message brokers and an implementation of a `Worker` that should work
//! well enough in most situations:
//!
//! * [RabbitMQ](rabbitmq/index.html)
//! * Faktory
//! * Amazon SQS
//!
//! # Example
//!
//! ```rust
//! extern crate batch;
//! extern crate batch_rabbitmq;
//! extern crate tokio;
//!
//! use batch::{job, Query};
//! use batch_rabbitmq::{self, queues};
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
//!     let conn = batch_rabbitmq::Connection::open("amqp://guest:guest@localhost/%2f")
//!         .and_then(|mut client| client.declare(Notifications).map(|_| client))
//!         .and_then(|client| {
//!             let job = say_hello(String::from("Ferris"));
//!             Notifications(job).dispatch(&client)
//!         });
//!
//! # if false {
//!     tokio::run(
//!         send.map_err(|e| eprintln!("Couldn't publish message: {}", e))
//!     );
//! # }
//! }
//! ```

#![doc(html_root_url = "https://docs.rs/batch/0.2.0")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

#[cfg(feature = "codegen")]
extern crate batch_codegen;
extern crate failure;
extern crate futures;
extern crate log;
extern crate serde;
extern crate serde_json;
extern crate tokio_executor;
extern crate uuid;
extern crate wait_timeout;

mod client;
mod consumer;
mod delivery;
mod dispatch;
pub mod dsl;
/// Not public API.
#[doc(hidden)]
pub mod export;
mod factory;
mod job;
mod query;
mod queue;
mod worker;

#[cfg(feature = "codegen")]
pub use batch_codegen::job;
pub use client::Client;
pub use consumer::{Consumer, ToConsumer};
pub use delivery::Delivery;
pub use dispatch::Dispatch;
pub use factory::Factory;
pub use job::{Job, Properties};
pub use query::Query;
pub use queue::Queue;
pub use worker::Worker;

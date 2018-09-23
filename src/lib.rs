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
//! #[macro_use]
//! extern crate serde;
//! extern crate tokio;
//!
//! use batch::{job, Declare};
//! use batch::rabbitmq::{self, exchanges};
//! use tokio::prelude::*;
//!
//! exchanges! {
//!     Notifications {
//!         name = "notifications"
//!     }
//! }
//!
//! #[job(name = "batch-example.say-hello")]
//! fn say_hello(name: String) {
//!     println!("Hello {}!", name);
//! }
//!
//! fn main() {
//!     let conn = rabbitmq::Connection::open("amqp://guest:guest@localhost/");
//!     let notifications = conn.and_then(|mut conn| Notifications::declare(&mut conn));
//!     let send = notifications.and_then(|notifications| {
//!         use batch::dsl::*;
//!
//!         notifications.with(say_hello(String::from("Ferris"))).deliver()
//!     });
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
extern crate tokio_executor;
extern crate wait_timeout;
extern crate uuid;

mod consumer;
mod declare;
mod delivery;
mod dispatch;
pub mod dsl;
/// Not public API.
#[doc(hidden)]
pub mod export;
mod factory;
mod job;
mod publisher;
mod worker;

#[cfg(feature = "codegen")]
pub use batch_codegen::job;
pub use consumer::{Consumer, ToConsumer};
pub use declare::{Callbacks, Declarator, Declare, DeclareMarker};
pub use delivery::Delivery;
pub use dispatch::Dispatch;
pub use factory::Factory;
pub use job::{Job, Priority, Properties};
pub use publisher::Publisher;
pub use worker::Worker;

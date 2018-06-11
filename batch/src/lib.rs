//! Batch is a distributed job queue/task queue library.
//!
//! This library allows you to send a job to a RabbitMQ broker, so that a worker will be able
//! to pull it and execute the associated handler. It leverages the `futures` and `tokio-core`
//! crates to provide asynchronous I/O operations.
//!
//! # Example
//!
//! ```rust
//! #[macro_use]
//! extern crate batch;
//! # extern crate failure;
//! extern crate futures;
//! #[macro_use]
//! extern crate lazy_static;
//! #[macro_use]
//! extern crate serde;
//! extern crate tokio;
//!
//! use batch::{exchange, job, ClientBuilder};
//! # use failure::Error;
//! use futures::Future;
//!
//! #[derive(Serialize, Deserialize, Job)]
//! #[job_routing_key = "hello-world"]
//! struct SayHello {
//!     to: String,
//! }
//!
//! fn main() {
//!     let exchanges = vec![
//!         exchange("batch.examples"),
//!     ];
//!     let client = ClientBuilder::new()
//!         .connection_url("amqp://localhost/%2f")
//!         .exchanges(exchanges)
//!         .build();
//!     let send = client.and_then(|client| {
//!         let to = "Ferris".to_string();
//!
//!         job(SayHello { to }).exchange("batch.example").send(&client)
//!     }).map_err(|e| eprintln!("Couldn't publish message: {}", e));
//!
//! # if false {
//!     tokio::run(send);
//! # }
//! }
//! ```

#![doc(html_root_url = "https://docs.rs/batch/0.1.1")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![allow(unused_imports)]
#![allow(unknown_lints)]

extern crate amq_protocol;
extern crate bytes;
#[cfg(test)]
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate lapin_futures as lapin;
#[macro_use]
extern crate log;
extern crate native_tls;
extern crate num_cpus;
#[macro_use]
extern crate serde;
extern crate serde_json;
#[cfg(test)]
extern crate tokio;
extern crate tokio_executor;
extern crate tokio_io;
extern crate tokio_reactor;
extern crate tokio_tcp;
extern crate tokio_tls;
extern crate uuid;
extern crate wait_timeout;

#[cfg(feature = "codegen")]
#[macro_use]
extern crate batch_codegen;

#[cfg(feature = "codegen")]
#[doc(hidden)]
pub use batch_codegen::*;

use serde_json::de;
use serde_json::ser;

mod client;
mod error;
mod job;
mod query;
mod rabbitmq;
mod worker;

pub use client::{Client, ClientBuilder};
pub use error::Error;
pub use job::{Job, Perform, Priority};
pub use query::{job, Query};
pub use rabbitmq::{exchange, queue, ExchangeBuilder, QueueBuilder};
pub use worker::{Worker, WorkerBuilder};

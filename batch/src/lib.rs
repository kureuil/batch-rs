//! Batch is a distributed task queue library.
//!
//! This library allows you to send a task to a RabbitMQ broker, so that a worker will be able
//! to pull it and execute the associated handler. It leverages the `futures` and `tokio-core`
//! crates to provide asynchronous I/O operations.
//!
//! # Example
//!
//! ```rust
//! #[macro_use]
//! extern crate batch;
//! extern crate futures;
//! #[macro_use]
//! extern crate serde;
//! extern crate tokio_core;
//!
//! use batch::{exchange, job, ClientBuilder};
//! use futures::Future;
//! use tokio_core::reactor::Core;
//!
//! #[derive(Serialize, Deserialize, Task)]
//! #[task_routing_key = "hello-world"]
//! struct SayHello {
//!     to: String,
//! }
//!
//! fn main() {
//!     let mut core = Core::new().unwrap();
//!     let handle = core.handle();
//!
//!     let exchanges = vec![
//!         exchange("batch.examples"),
//!     ];
//!     let client = ClientBuilder::new()
//!         .connection_url("amqp://localhost/%2f")
//!         .exchanges(exchanges)
//!         .handle(handle)
//!         .build();
//!     let send = client.and_then(|client| {
//!         let task = SayHello {
//!             to: "Ferris".into(),
//!         };
//!
//!         job(task).exchange("batch.example").send(&client)
//!     });
//!
//!     core.run(send).unwrap();
//! }
//! ```

#![doc(html_root_url = "https://docs.rs/batch/0.1.0")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![allow(unused_imports)]
#![allow(unknown_lints)]

#[macro_use]
extern crate failure;
extern crate futures;
extern crate lapin_async;
extern crate lapin_futures as lapin;
extern crate lapin_futures_rustls as lapin_rustls;
extern crate lapin_futures_tls_api as lapin_tls_api;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;
extern crate serde_json;
extern crate tokio_core;
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
mod rabbitmq;
mod task;
mod worker;

pub use client::{Client, ClientBuilder};
pub use error::Error;
pub use job::{job, Query};
pub use rabbitmq::{exchange, queue, ExchangeBuilder, QueueBuilder};
pub use task::{Perform, Task};
pub use worker::{Worker, WorkerBuilder};

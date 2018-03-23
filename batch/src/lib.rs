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
//! # extern crate failure;
//! extern crate futures;
//! extern crate serde;
//! #[macro_use]
//! extern crate serde_derive;
//! extern crate tokio_core;
//!
//! use batch::{exchange, job, ClientBuilder};
//! # use failure::Error;
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
//! #   example().unwrap();
//! # }
//! #
//! # fn example() -> Result<(), Error> {
//!     let mut core = Core::new()?;
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
//! # if false {
//!     core.run(send)?;
//! # }
//! # Ok(())
//! }
//! ```

#![doc(html_root_url = "https://docs.rs/batch/0.1.1")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![allow(unused_imports)]
#![allow(unknown_lints)]

extern crate amq_protocol;
extern crate bytes;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate lapin_async;
extern crate lapin_futures as lapin;
#[macro_use]
extern crate log;
extern crate native_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate tokio_io;
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
mod rabbitmq;
mod task;
mod worker;

pub use client::{Client, ClientBuilder};
pub use error::Error;
pub use job::{job, Query};
pub use rabbitmq::{exchange, queue, ExchangeBuilder, QueueBuilder};
pub use task::{Perform, Priority, Task};
pub use worker::{Worker, WorkerBuilder};

//! RabbitMQ adapter for the `batch` library.

#![doc(html_root_url = "https://docs.rs/batch-rabbitmq/0.2.0")]
#![deny(missing_debug_implementations)]
// #![deny(missing_docs)]

extern crate amq_protocol;
extern crate batch;
#[cfg(feature = "codegen")]
extern crate batch_rabbitmq_codegen;
extern crate bytes;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate futures;
extern crate lapin_futures as lapin;
#[macro_use]
extern crate log;
extern crate native_tls;
#[macro_use]
extern crate serde;
extern crate serde_json;
extern crate tokio_executor;
extern crate tokio_io;
extern crate tokio_reactor;
extern crate tokio_tcp;
extern crate tokio_tls;
extern crate uuid;

mod connection;
mod consumer;
mod declare;
mod delivery;
mod exchange;
/// Not public API.
#[doc(hidden)]
pub mod export;
mod queue;
mod stream;

#[cfg(feature = "codegen")]
pub use batch_rabbitmq_codegen::queues;
pub use connection::{Builder as ConnectionBuilder, Connection};
pub use declare::Declare;
pub use delivery::{Acknowledge, Delivery, Reject};
pub use exchange::{Builder as ExchangeBuilder, Exchange, Kind as ExchangeKind};
pub use queue::{Builder as QueueBuilder, Queue};

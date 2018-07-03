//! RabbitMQ adapter for the `batch` library.

#![doc(html_root_url = "https://docs.rs/batch-rabbitmq/0.2.0")]
#![deny(missing_debug_implementations)]
// #![deny(missing_docs)]

extern crate amq_protocol;
extern crate batch_core;
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
mod delivery;
mod dispatch;
mod exchange;
mod query;
mod queue;
mod stream;

pub use connection::Connection;
pub use delivery::{Acknowledge, Delivery, Reject};
pub use exchange::{Builder as ExchangeBuilder, Exchange, Kind as ExchangeKind};
pub use query::Query;
pub use queue::{Builder as QueueBuilder, Queue};

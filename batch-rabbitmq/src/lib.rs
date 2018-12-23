//! RabbitMQ adapter for the `batch` library.

#![doc(html_root_url = "https://docs.rs/batch-rabbitmq/0.2.0")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod connection;
mod consumer;
mod declare;
mod delivery;
mod exchange;
/// Not public API. This module is exempt from any semver guarantees.
#[doc(hidden)]
pub mod export;
mod queue;
mod stream;

pub use crate::connection::{Builder as ConnectionBuilder, ConnectFuture, Connection, SendFuture};
pub use crate::declare::Declare;
pub use crate::delivery::{AcknowledgeFuture, Delivery, RejectFuture};
pub use crate::exchange::{Builder as ExchangeBuilder, Exchange, Kind as ExchangeKind};
pub use crate::queue::{Builder as QueueBuilder, Queue};
#[cfg(feature = "codegen")]
pub use batch_rabbitmq_codegen::queues;

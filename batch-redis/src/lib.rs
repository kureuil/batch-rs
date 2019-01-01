//! RabbitMQ adapter for the `batch` library.

#![doc(html_root_url = "https://docs.rs/batch-redis/0.2.0")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod connection;
mod consumer;
mod delivery;
/// Not public API. This module is exempt from any semver guarantees.
#[doc(hidden)]
pub mod export;

// #[cfg(feature = "codegen")]
// pub use batch_redis_codegen::queues;
pub use crate::connection::{Connection, OpenError, OpenFuture, SendFuture};

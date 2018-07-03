//! Core traits and types of the batch library.
//!
//! This crate is meant for library authors that want to leverage batch capabilities. If you're an application author
//! (or are unsure), you should probably be using the `batch` crate instead.

#![doc(html_root_url = "https://docs.rs/batch-core/0.2.0")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

extern crate failure;
extern crate futures;
#[macro_use]
extern crate serde;
extern crate serde_json;
extern crate uuid;

mod consumer;
mod declare;
mod delivery;
mod dispatch;
pub mod dsl;
mod factory;
mod job;
mod publisher;

pub use consumer::{Consumer, ToConsumer};
pub use declare::{Callbacks, Declarator, Declare};
pub use delivery::Delivery;
pub use dispatch::Dispatch;
pub use factory::Factory;
pub use job::{Job, Priority, Properties};
pub use publisher::{Publisher, ToPublisher};

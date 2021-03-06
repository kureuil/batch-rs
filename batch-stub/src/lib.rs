//! Testing utilities for the Batch library.
//!
//! This crate exists to help you test your code depending on Batch. For now it only contains a
//! [`batch::Client`] implementation but we'd like to have more testing facilities in the
//! future.

#![doc(html_root_url = "https://docs.rs/batch-stub/0.2.0")]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod client;

pub use crate::client::Client;

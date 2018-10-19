//! Procedural macros for the batch_rabbitmq crate.

#![recursion_limit = "256"]
#![doc(html_root_url = "https://docs.rs/batch-rabbitmq-codegen/0.2.0")]
#![deny(missing_debug_implementations)]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;

mod error;
mod queues;

/// A procedural macro to declare queues.
///
/// This macro should be written as a top-level item in a module, as it expands to statements. For
/// each declared queue, it will create a structure and a function named after the given
/// identifier, and implementations for the `batch::Queue` and `batch_rabbitmq::Declare`.
///
/// # Example
///
/// ```ignore
/// queues! {
///     Maintenance {
///         name = "maintenance",
///         bindings = [
///             super::jobs::prune_database,
///         ],
///     }
///
///     Transcoding {
///         name = "transcoding",
///         exchange = "my-example",
///         bindings = [
///             super::jobs::convert_video_file,
///         ],
///     }
/// }
/// ```
#[proc_macro]
pub fn queues(input: TokenStream) -> TokenStream {
    queues::impl_macro(input)
}

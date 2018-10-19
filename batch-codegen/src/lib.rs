//! Batch procedural macros
//!
//! This crate exposes general purpose macros for the Batch background job library. There are
//! currently one macro exported by this crate:
//!
//! * [`job`]: a macro used to generate Batch compliant `Job` implementations from raw functions
//!
//! [`job`]: fn.job.html

#![recursion_limit = "256"]
#![doc(html_root_url = "https://docs.rs/batch-codegen/0.2.0")]
#![deny(missing_debug_implementations)]

extern crate humantime;
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

mod error;
mod job;

use proc_macro::TokenStream;

/// A procedural macro to declare job based on functions.
///
/// This attribute is meant to be used to annotate functions and transform them in `Job`s. It
/// applies the following modifications to the annotated function:
///
/// - It creates a structure with the exact same name as the function, storing all the fields
///   that will be sent through the message broker. This structure then implements the `Job`
///   trait.
/// - It removes the parameters marked as injected from the function signature and changes the
///   return type to the generated job structure created as stated above.
///
/// If you wish to execute a job without going through a message broker, you can call the
/// `perform_now` method on the job returned by the annotated function. This method takes as
/// parameter all of the injected parameters.
///
/// This macro takes parameters in the form `$key = $value` where `$key` is the name of the
/// parameter written as an identifier (which means no quotes around it name), and where `$value`
/// is the value associated to the given key and can be either an integer, a string, an identifier
/// or an array of any of the previous types.
///
/// # Parameters
///
/// - `name`: `Identifier`  
///   This parameter is used to set the name of the job.
/// - `wrapper`: `Option<Identifier>`  
///   This parameter is used to set the name of the job structure that will be generated. By
///   default the wrapper is named after the annotated function.
/// - `inject`: `Option<[Identifier]>`  
///   A list of the parameters that will be injected at run-time. By default, none of the
///   parameters are to be injected.
/// - `retries`: `Option<u32>`  
///   Number of times this job will be retried.
/// - `timeout`: `Option<String>`  
///   A string representing the duration the job will be allowed to run. Must in a format
///   accepted by the [`humantime`] crate.
/// - `priority`: `Option<Identifier>`  
///   The priority associated to the job. Can be any of `trivial`, `low`, `normal`, `high`,
///   `critical`. Default is `normal`.
///
/// [`humantime`]: https://docs.rs/humantime/1/humantime/fn.parse_duration.html
///
/// # Example
///
/// ```ignore
/// #[job(name = "batch-example.say-hello", timeout = "5min 30sec", priority = low)]
/// fn say_hello(name: String) {
///     println!("Hello {}", name);
/// }
/// ```
#[proc_macro_attribute]
pub fn job(args: TokenStream, input: TokenStream) -> TokenStream {
    job::impl_macro(args, input)
}

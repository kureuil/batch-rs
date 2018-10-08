//! Batch procedural macros
//!
//! This crate exposes general purpose macros for the Batch background job library. There are
//! currently one macro exported by this crate:
//!
//! * [`job`]: a macro used to generate Batch compliant `Job` implementations from raw functions
//!
//! [`job`]: fn.job.html

#![recursion_limit = "256"]

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

#[proc_macro_attribute]
pub fn job(args: TokenStream, input: TokenStream) -> TokenStream {
    job::impl_macro(args, input)
}

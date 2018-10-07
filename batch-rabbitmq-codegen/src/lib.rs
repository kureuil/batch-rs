#![recursion_limit = "256"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;

mod error;
mod queues;

#[proc_macro]
pub fn queues(input: TokenStream) -> TokenStream {
    queues::impl_macro(input)
}

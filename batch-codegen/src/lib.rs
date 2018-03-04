//! This crate provides a Batch's derive macro.
//!
//! ```rust,ignore
//! #[derive(Task)]
//! ```

#![doc(html_root_url = "https://docs.rs/batch-codegen/0.1.0")]
#![deny(missing_debug_implementations)]
#![recursion_limit = "128"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::{ToTokens, Tokens};
use syn::{DeriveInput, Ident, Lit, Meta};

/// Macros 1.1 implementation of `#[derive(Task)]`
///
/// This macro supports several attributes:
///
/// * `task_name`: a unique ID for the task.
///   e.g: `#[task_name = "batch-rs:send-confirmation-email"]`
///   **default value**: The derived struct name
/// * `task_exchange`: the exchange this task will be published to.
///   e.g: `#[task_exchange = "batch.example"]`
///   **default value**: `""`
/// * `task_routing_key`: the routing key associated to the task.
///   e.g: `#[task_routing_key = "mailer"]`
/// * `task_timeout`: Number of seconds available for the task to execute. If the time limit is
///   exceeded, the task's process is killed and the task is marked as failed.
///   e.g: `#[task_timeout = "120"]`
///   **default value**: `900` (15 minutes)
/// * `task_retries`: Number of times the task should be retried in case of error.
///   e.g: `#[task_retries = "5"]`
///   **default value**: `2`
/// * `task_priority`: The priority associated to the task
///   e.g: `#[task_priority = "critical"]`
///   **default value**: `"normal"`
#[proc_macro_derive(Task,
                    attributes(task_name, task_exchange, task_routing_key, task_timeout,
                               task_retries, task_priority))]
pub fn task_derive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let task_name = get_derive_name_attr(&input);
    let task_exchange = get_derive_exchange_attr(&input);
    let task_routing_key = get_derive_routing_key_attr(&input);
    let task_timeout = get_derive_timeout_attr(&input);
    let task_retries = get_derive_retries_attr(&input);
    let task_priority = get_derive_priority_attr(&input);
    let name = &input.ident;

    let expanded = quote! {
        impl ::batch::Task for #name {

            fn name() -> &'static str {
                #task_name
            }

            fn exchange() -> &'static str {
                #task_exchange
            }

            fn routing_key() -> &'static str {
                #task_routing_key
            }

            fn timeout() -> Option<::std::time::Duration> {
                #task_timeout
            }

            fn retries() -> u32 {
                #task_retries
            }

            fn priority() -> ::batch::Priority {
                #task_priority
            }
        }
    };
    expanded.into()
}

fn get_derive_name_attr(input: &DeriveInput) -> Tokens {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "task_name");
        raw.unwrap_or_else(|| input.ident.as_ref().to_string())
    };
    attr.into_tokens()
}

fn get_derive_exchange_attr(input: &DeriveInput) -> Tokens {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "task_exchange");
        raw.unwrap_or_else(|| "".to_string())
    };
    attr.into_tokens()
}

fn get_derive_routing_key_attr(input: &DeriveInput) -> Tokens {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "task_routing_key");
        raw.expect("task_routing_key is a mandatory attribute when deriving Task")
    };
    attr.into_tokens()
}

fn get_derive_timeout_attr(input: &DeriveInput) -> Tokens {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "task_timeout");
        raw.unwrap_or_else(|| "900".to_string())
    };
    let timeout = attr.parse::<u64>()
        .expect("Couldn't parse timeout as an unsigned integer");
    quote! {
        ::std::option::Option::Some(::std::time::Duration::from_secs(#timeout))
    }
}

fn get_derive_retries_attr(input: &DeriveInput) -> Tokens {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "task_retries");
        raw.unwrap_or_else(|| "2".to_string())
    };
    let retries = attr.parse::<u32>()
        .expect("Couldn't parse retries as an unsigned integer");
    quote! {
        #retries
    }
}

fn get_derive_priority_attr(input: &DeriveInput) -> Tokens {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "task_priority");
        raw.unwrap_or_else(|| "normal".to_string())
    };
    match attr.to_lowercase().as_ref() {
        "trivial" => quote! { ::batch::Priority::Trivial },
        "low" => quote! { ::batch::Priority::Low },
        "normal" => quote! { ::batch::Priority::Normal },
        "high" => quote! { ::batch::Priority::High },
        "critical" => quote! { ::batch::Priority::Critical },
        _ => {
            panic!("Invalid priority, must be one of: trivial, low, normal, high, critical.");
        }
    }
}

/// Gets the string value of an attribute by its name.
fn get_str_attr_by_name(haystack: &[syn::Attribute], needle: &str) -> Option<String> {
    let attr = get_raw_attr_by_name(haystack, needle);
    attr.and_then(|attr| {
        if let Lit::Str(literal) = attr {
            Some(literal.value())
        } else {
            None
        }
    })
}

/// Gets the raw value of an attribute by its name.
fn get_raw_attr_by_name(haystack: &[syn::Attribute], needle_raw: &str) -> Option<Lit> {
    let needle = Ident::from(needle_raw);
    for attr in haystack {
        let meta = match attr.interpret_meta() {
            Some(meta) => meta,
            None => continue,
        };
        let nv = match meta {
            Meta::NameValue(nv) => nv,
            _ => continue,
        };
        if nv.ident != needle {
            continue;
        }
        return Some(nv.lit.clone());
    }
    None
}

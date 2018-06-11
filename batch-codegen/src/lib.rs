//! This crate provides a Batch's derive macro.
//!
//! ```rust,ignore
//! #[derive(Job)]
//! ```

#![doc(html_root_url = "https://docs.rs/batch-codegen/0.1.0")]
#![deny(missing_debug_implementations)]
#![recursion_limit = "128"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream as StdTokenStream;
use proc_macro2::{Span, TokenStream};
use syn::{DeriveInput, Ident, Lit, Meta};

/// Macros 1.1 implementation of `#[derive(Job)]`
///
/// This macro supports several attributes:
///
/// * `job_name`: a unique ID for the job.
///   e.g: `#[job_name = "batch-rs:send-confirmation-email"]`
///   **default value**: The derived struct name
/// * `job_exchange`: the exchange this job will be published to.
///   e.g: `#[job_exchange = "batch.example"]`
///   **default value**: `""`
/// * `job_routing_key`: the routing key associated to the job.
///   e.g: `#[job_routing_key = "mailer"]`
/// * `job_timeout`: Number of seconds available for the job to execute. If the time limit is
///   exceeded, the job's process is killed and the job is marked as failed.
///   e.g: `#[job_timeout = "120"]`
///   **default value**: `900` (15 minutes)
/// * `job_retries`: Number of times the job should be retried in case of error.
///   e.g: `#[job_retries = "5"]`
///   **default value**: `2`
/// * `job_priority`: The priority associated to the job
///   e.g: `#[job_priority = "critical"]`
///   **default value**: `"normal"`
#[proc_macro_derive(
    Job, attributes(job_name, job_exchange, job_routing_key, job_timeout, job_retries, job_priority)
)]
pub fn task_derive(input: StdTokenStream) -> StdTokenStream {
    let input: DeriveInput = syn::parse(input.into()).unwrap();
    let job_name = get_derive_name_attr(&input);
    let job_exchange = get_derive_exchange_attr(&input);
    let job_routing_key = get_derive_routing_key_attr(&input);
    let job_timeout = get_derive_timeout_attr(&input);
    let job_retries = get_derive_retries_attr(&input);
    let job_priority = get_derive_priority_attr(&input);
    let name = &input.ident;
    let impl_block_name = gen_derive_impl_block_name(name.to_string());

    let expanded = quote! {
        #[allow(non_upper_case_globals)]
        const #impl_block_name: () =
        {
            extern crate batch as _batch;

            use ::std::string::String;
            use ::std::option::Option;
            use ::std::time::Duration;

            lazy_static! {
                static ref _BATCH_JOB_NAME: String = #job_name.replace("::", ".");
            }

            impl _batch::Job for #name {
                fn name() -> &'static str {
                    _BATCH_JOB_NAME.as_ref()
                }

                fn exchange() -> &'static str {
                    #job_exchange
                }

                fn routing_key() -> &'static str {
                    #job_routing_key
                }

                fn timeout() -> Option<Duration> {
                    #job_timeout
                }

                fn retries() -> u32 {
                    #job_retries
                }

                fn priority() -> _batch::Priority {
                    #job_priority
                }
            }
        };
    };
    expanded.into()
}

fn get_derive_name_attr(input: &DeriveInput) -> TokenStream {
    if let Some(raw) = get_str_attr_by_name(&input.attrs, "job_name") {
        quote! { #raw }
    } else {
        let name = input.ident.to_string();
        quote! {
            concat!(concat!(module_path!(), "::"), #name)
        }
    }
}

fn get_derive_exchange_attr(input: &DeriveInput) -> TokenStream {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "job_exchange");
        raw.unwrap_or_else(|| "".to_string())
    };
    quote! { #attr }
}

fn get_derive_routing_key_attr(input: &DeriveInput) -> TokenStream {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "job_routing_key");
        raw.expect("job_routing_key is a mandatory attribute when deriving Job")
    };
    quote! { #attr }
}

fn get_derive_timeout_attr(input: &DeriveInput) -> TokenStream {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "job_timeout");
        raw.unwrap_or_else(|| "900".to_string())
    };
    let timeout = attr.parse::<u64>()
        .expect("Couldn't parse timeout as an unsigned integer");
    quote! {
        Option::Some(Duration::from_secs(#timeout))
    }
}

fn get_derive_retries_attr(input: &DeriveInput) -> TokenStream {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "job_retries");
        raw.unwrap_or_else(|| "2".to_string())
    };
    let retries = attr.parse::<u32>()
        .expect("Couldn't parse retries as an unsigned integer");
    quote! {
        #retries
    }
}

fn get_derive_priority_attr(input: &DeriveInput) -> TokenStream {
    let attr = {
        let raw = get_str_attr_by_name(&input.attrs, "job_priority");
        raw.unwrap_or_else(|| "normal".to_string())
    };
    match attr.to_lowercase().as_ref() {
        "trivial" => quote! { _batch::Priority::Trivial },
        "low" => quote! { _batch::Priority::Low },
        "normal" => quote! { _batch::Priority::Normal },
        "high" => quote! { _batch::Priority::High },
        "critical" => quote! { _batch::Priority::Critical },
        _ => {
            panic!("Invalid priority, must be one of: trivial, low, normal, high, critical.");
        }
    }
}

fn gen_derive_impl_block_name(name: String) -> TokenStream {
    let ident = Ident::new(&format!("_IMPL_BATCH_JOB_FOR_{}", name), Span::call_site());
    quote! { #ident }
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
fn get_raw_attr_by_name(haystack: &[syn::Attribute], needle: &str) -> Option<Lit> {
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

use std::collections::HashSet;

use proc_macro;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn;
use syn::parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;

use error::Error;

#[derive(Clone)]
struct JobAttrs {
    attrs: Vec<JobAttr>,
}

#[derive(Clone)]
enum JobAttr {
    Name(syn::LitStr),
    Wrapper(syn::Ident),
    Inject(HashSet<syn::Ident>),
    Retries(syn::LitInt),
    Timeout(syn::LitInt),
    Priority(Priority)
}

#[derive(Clone)]
enum Priority {
    Trivial(priorities::trivial),
    Low(priorities::low),
    Normal(priorities::normal),
    High(priorities::high),
    Critical(priorities::critical),
}

#[derive(Clone)]
struct Job {
    errors: Vec<Error>,
    visibility: syn::Visibility,
    name: String,
    wrapper: Option<syn::Ident>,
    retries: Option<syn::LitInt>,
    timeout: Option<syn::LitInt>,
    priority: Option<Priority>,
    injected: HashSet<syn::Ident>,
    injected_args: Vec<syn::FnArg>,
    serialized_args: Vec<syn::FnArg>,
    original_args: Vec<syn::FnArg>,
    inner_block: Option<syn::Block>,
    ret: Option<syn::Type>,
}

impl JobAttrs {
    fn name(&self) -> Option<String> {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                JobAttr::Name(s) => Some(s.value()),
                _ => None,
            })
            .next()
    }

    fn wrapper(&self) -> Option<syn::Ident> {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                JobAttr::Wrapper(i) => Some(i.clone()),
                _ => None,
            })
            .next()
    }

    fn inject(&self) -> HashSet<syn::Ident> {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                JobAttr::Inject(i) => Some(i.clone()),
                _ => None,
            })
            .next()
            .unwrap_or_else(HashSet::new)
    }

    fn retries(&self) -> Option<syn::LitInt> {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                JobAttr::Retries(r) => Some(r.clone()),
                _ => None,
            })
            .next()
    }

    fn timeout(&self) -> Option<syn::LitInt> {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                JobAttr::Timeout(t) => Some(t.clone()),
                _ => None,
            })
            .next()
    }

    fn priority(&self) -> Option<Priority> {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                JobAttr::Priority(p) => Some(p.clone()),
                _ => None,
            })
            .next()
    }
}

impl parse::Parse for JobAttrs {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let attrs: Punctuated<_, Token![,]> = input.parse_terminated(JobAttr::parse)?;
        Ok(JobAttrs {
            attrs: attrs.into_iter().collect(),
        })
    }
}

mod kw {
    custom_keyword!(name);
    custom_keyword!(wrapper);
    custom_keyword!(inject);
    custom_keyword!(retries);
    custom_keyword!(timeout);
    custom_keyword!(priority);
}

impl parse::Parse for JobAttr {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::name) {
            input.parse::<kw::name>()?;
            input.parse::<Token![=]>()?;
            Ok(JobAttr::Name(input.parse()?))
        } else if lookahead.peek(kw::wrapper) {
            input.parse::<kw::wrapper>()?;
            input.parse::<Token![=]>()?;
            Ok(JobAttr::Wrapper(input.parse()?))
        } else if lookahead.peek(kw::inject) {
            input.parse::<kw::inject>()?;
            input.parse::<Token![=]>()?;
            let content;
            let _: syn::token::Bracket = bracketed!(content in input);
            let injected: Punctuated<_, Token![,]> = content.parse_terminated(syn::Ident::parse)?;
            Ok(JobAttr::Inject(injected.into_iter().collect()))
        } else if lookahead.peek(kw::retries) {
            input.parse::<kw::retries>()?;
            input.parse::<Token![=]>()?;
            Ok(JobAttr::Retries(input.parse()?))
        } else if lookahead.peek(kw::timeout) {
            input.parse::<kw::timeout>()?;
            input.parse::<Token![=]>()?;
            Ok(JobAttr::Timeout(input.parse()?))
        } else if lookahead.peek(kw::priority) {
            input.parse::<kw::priority>()?;
            input.parse::<Token![=]>()?;
            Ok(JobAttr::Priority(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

mod priorities {
    custom_keyword!(trivial);
    custom_keyword!(low);
    custom_keyword!(normal);
    custom_keyword!(high);
    custom_keyword!(critical);
}

impl parse::Parse for Priority {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(priorities::trivial) {
            Ok(Priority::Trivial(input.parse::<priorities::trivial>()?))
        } else if lookahead.peek(priorities::low) {
            Ok(Priority::Low(input.parse::<priorities::low>()?))
        } else if lookahead.peek(priorities::normal) {
            Ok(Priority::Normal(input.parse::<priorities::normal>()?))
        } else if lookahead.peek(priorities::high) {
            Ok(Priority::High(input.parse::<priorities::high>()?))
        } else if lookahead.peek(priorities::critical) {
            Ok(Priority::Critical(input.parse::<priorities::critical>()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for Priority {
    fn to_tokens(&self, dst: &mut TokenStream) {
        let tokens = match *self {
            Priority::Trivial(ref p) => quote_spanned!(p.span() => ::batch::Priority::Trivial),
            Priority::Low(ref p) => quote_spanned!(p.span() => ::batch::Priority::Low),
            Priority::Normal(ref p) => quote_spanned!(p.span() => ::batch::Priority::Normal),
            Priority::High(ref p) => quote_spanned!(p.span() => ::batch::Priority::High),
            Priority::Critical(ref p) => quote_spanned!(p.span() => ::batch::Priority::Critical),
        };
        dst.extend(tokens);
    }
}

impl Job {
    fn new(attrs: JobAttrs) -> Result<Self, Error> {
        const ERR_MISSING_NAME: &str = "missing mandatory name attribute";

        let errors = Vec::new();
        let visibility = syn::Visibility::Inherited;
        let name = match attrs.name() {
            Some(name) => name,
            None => return Err(Error::new(ERR_MISSING_NAME)),
        };
        let wrapper = attrs.wrapper();
        let retries = attrs.retries();
        let timeout = attrs.timeout();
        let priority = attrs.priority();
        let injected = attrs.inject();
        let injected_args = Vec::new();
        let serialized_args = Vec::new();
        let original_args = Vec::new();
        let inner_block = None;
        let ret = None;
        Ok(Job {
            errors,
            visibility,
            name,
            wrapper,
            retries,
            timeout,
            priority,
            injected,
            injected_args,
            serialized_args,
            original_args,
            inner_block,
            ret,
        })
    }
}

impl VisitMut for Job {
    fn visit_item_fn_mut(&mut self, node: &mut syn::ItemFn) {
        const ERR_ABI: &str = "functions with non-Rust ABI are not supported";

        self.visibility = node.vis.clone();
        if let Some(ref mut it) = node.abi {
            self.errors.push(Error::spanned(ERR_ABI, it.span()));
        };
        if self.wrapper.is_none() {
            let ident = node.ident.clone();
            self.wrapper = Some(ident);
        }
        self.visit_fn_decl_mut(&mut *node.decl);
        self.inner_block = Some((*node.block).clone());
        let wrapper = self.wrapper.as_ref().unwrap();
        let serialized_fields = self
            .serialized_args
            .iter()
            .fold(TokenStream::new(), |acc, arg| match arg {
                syn::FnArg::Captured(cap) => match cap.pat {
                    syn::Pat::Ident(ref pat) => {
                        let ident = &pat.ident;
                        quote! {
                            #acc
                            #ident,
                        }
                    }
                    _ => acc,
                },
                _ => acc,
            });
        node.block = Box::new(parse_quote!({
            #wrapper {
                #serialized_fields
            }
        }));
    }

    fn visit_fn_decl_mut(&mut self, node: &mut syn::FnDecl) {
        const ERR_GENERICS: &str = "functions with generic arguments are not supported";
        const ERR_UNKOWN_INJECT: &str =
            "an inject parameter should match the name of one of the functin's argument";
        const ERR_VARIADIC: &str = "functions with variadic arguments are not supported";

        if node.generics.params.len() > 0 {
            self.errors
                .push(Error::spanned(ERR_GENERICS, node.generics.span()));
        }
        let (serialized, injected) = node
            .inputs
            .clone()
            .into_iter()
            .partition::<Vec<syn::FnArg>, _>(|arg| match arg {
                syn::FnArg::Captured(captured) => {
                    if let syn::Pat::Ident(ref pat) = captured.pat {
                        !self.injected.remove(&pat.ident)
                    } else {
                        true
                    }
                }
                _ => true,
            });
        for inject in &self.injected {
            self.errors
                .push(Error::spanned(ERR_UNKOWN_INJECT, inject.span()))
        }
        self.serialized_args = serialized.clone();
        self.injected_args = injected;
        self.original_args = node.inputs.clone().into_iter().collect();
        node.inputs = serialized.into_iter().collect();
        if let Some(ref mut it) = node.variadic {
            self.errors.push(Error::spanned(ERR_VARIADIC, it.span()));
        }
        if let syn::ReturnType::Type(_arr, ref ty) = node.output {
            self.ret = Some((**ty).clone());
        }
        // Unwrapping is safe here because we did set it while visiting `ItemFn`.
        let wrapper = self.wrapper.as_ref().unwrap();
        node.output = parse_quote!(-> #wrapper);
    }
}

fn args2fields<'a>(args: impl IntoIterator<Item = &'a syn::FnArg>) -> TokenStream {
    args.into_iter()
        .fold(TokenStream::new(), |acc, arg| match arg {
            syn::FnArg::Captured(cap) => {
                let ident = match cap.pat {
                    syn::Pat::Ident(ref pat) => &pat.ident,
                    _ => return acc,
                };
                let ty = &cap.ty;
                quote! {
                    #acc
                    #ident: #ty,
                }
            }
            _ => acc,
        })
}

impl ToTokens for Job {
    fn to_tokens(&self, dst: &mut TokenStream) {
        let vis = &self.visibility;
        let wrapper = self.wrapper.as_ref().unwrap();
        let retries = self.retries.as_ref().map(|r| quote! {
            fn retries(&self) -> u32 {
                #r
            }
        });
        let timeout = self.timeout.as_ref().map(|t| quote! {
            fn timeout(&self) -> ::batch::export::Duration {
                ::batch::export::Duration::from_secs(#t)
            }
        });
        let priority = self.priority.as_ref().map(|p| quote! {
            fn priority(&self) -> ::batch::Priority {
                #p
            }
        });
        let job_name = &self.name;
        let serialized_fields = args2fields(&self.serialized_args);
        let deserialized_bindings = self.serialized_args.iter().fold(
            TokenStream::new(),
            |acc, arg| match arg {
                syn::FnArg::Captured(cap) => match cap.pat {
                    syn::Pat::Ident(ref pat) => {
                        let ident = &pat.ident;
                        quote! {
                            #acc
                            let #ident = self.#ident;
                        }
                    }
                    _ => acc,
                },
                _ => acc,
            },
        );
        let injected_fields = args2fields(&self.injected_args);
        let injected_bindings = self
            .injected_args
            .iter()
            .fold(TokenStream::new(), |acc, arg| match arg {
                syn::FnArg::Captured(cap) => {
                    let ident = match cap.pat {
                        syn::Pat::Ident(ref pat) => &pat.ident,
                        _ => return acc,
                    };
                    let ty = &cap.ty;
                    quote! {
                        #acc
                        let #ident: #ty = _ctx.instantiate().unwrap();
                    }
                }
                _ => acc,
            });
        let injected_args = self.injected_args.iter().fold(
            TokenStream::new(),
            |acc, arg| match arg {
                syn::FnArg::Captured(cap) => match cap.pat {
                    syn::Pat::Ident(ref pat) => {
                        let ident = &pat.ident;
                        quote! {
                            #acc
                            #ident,
                        }
                    }
                    _ => acc,
                },
                _ => acc,
            },
        );
        let inner_block = {
            let block = &self.inner_block;
            quote!(#block)
        };
        let inner_invoke = quote!(self.perform_now(#injected_args));

        let ret_ty = self.ret.as_ref().map(|ty| quote!(#ty)).unwrap_or_else(|| quote!(()));
        let impl_test_into_future = self.ret.as_ref().map(|ty| quote_spanned!(ty.span() => impl_into_future::<#ty>()));

        let dummy_const = syn::Ident::new(
            &format!("__IMPL_BATCH_JOB_FOR_{}", wrapper.to_string()),
            Span::call_site(),
        );

        let wrapper_struct = quote! {
            #[derive(::batch::export::Deserialize, ::batch::export::Serialize)]
            #vis struct #wrapper {
                #serialized_fields
            }
        };

        let into_future = match self.ret {
            Some(ref _ty) => quote!({
                use ::batch::export::IntoFuture;
                IntoFuture::into_future(#inner_invoke)
            }),
            None => quote!({
                #inner_invoke;
                ::batch::export::ok(())
            }),
        };

        let output = quote! {
            #wrapper_struct

            const #dummy_const: () = {
                fn impl_into_future<T: ::batch::export::IntoFuture<Error = ::batch::export::Error>>() {}

                fn impl_test() {
                    #impl_test_into_future
                }

                impl #wrapper {
                    #vis fn perform_now(self, #injected_fields) -> #ret_ty {
                        #deserialized_bindings
                        #inner_block
                    }
                }

                impl ::batch::Job for #wrapper {
                    const NAME: &'static str = #job_name;

                    type PerformFuture = ::batch::export::Box<::batch::export::Future<Item = (), Error = ::batch::export::Error> + ::batch::export::Send>;

                    /// Performs the job.
                    ///
                    /// # Panics
                    ///
                    /// The function will panic if any parameter marked as `injected` cannot be found
                    /// in the given `Factory`.
                    fn perform(self, _ctx: ::batch::Factory) -> Self::PerformFuture {
                        #injected_bindings
                        ::batch::export::Box::new(#into_future)
                    }

                    #retries

                    #timeout

                    #priority
                }
            };
        };
        dst.extend(output);
    }
}

pub fn impl_macro(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attrs = parse_macro_input!(args as JobAttrs);
    let mut item = parse_macro_input!(input as syn::ItemFn);
    let mut job = match Job::new(attrs) {
        Ok(job) => job,
        Err(e) => return quote!(#e).into(),
    };
    job.visit_item_fn_mut(&mut item);
    if job.errors.len() > 0 {
        job.errors
            .iter()
            .fold(TokenStream::new(), |mut acc, err| {
                err.to_tokens(&mut acc);
                acc
            })
            .into()
    } else {
        let output = quote! {
            #job
            #item
        };
        output.into()
    }
}

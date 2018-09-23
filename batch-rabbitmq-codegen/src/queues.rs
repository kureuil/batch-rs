use proc_macro;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn;
use syn::parse;
use syn::punctuated::Punctuated;

use error::Error;

struct QueueAttrsList(Vec<QueueAttrs>);

#[derive(Clone)]
struct QueueAttrs {
    ident: syn::Ident,
    attrs: Vec<QueueAttr>,
}

#[derive(Clone)]
enum QueueAttr {
    Name(syn::LitStr),
    WithPriorities(syn::LitBool),
    Exclusive(syn::LitBool),
    Bindings(QueueBindings),
}

#[derive(Clone, Default)]
struct QueueBindings {
    bindings: Vec<QueueBinding>,
}

#[derive(Clone)]
struct QueueBinding {
    exchange: syn::Path,
    jobs: Vec<syn::Path>,
}

#[derive(Clone)]
struct Queue {
    ident: syn::Ident,
    name: String,
    with_priorities: bool,
    exclusive: bool,
    bindings: QueueBindings,
}

impl IntoIterator for QueueAttrsList {
    type Item = QueueAttrs;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl QueueAttrs {
    fn name(&self) -> Option<&syn::LitStr> {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                QueueAttr::Name(s) => Some(s),
                _ => None,
            })
            .next()
    }

    fn with_priorities(&self) -> bool {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                QueueAttr::WithPriorities(p) => Some(p.value),
                _ => None,
            })
            .next()
            .unwrap_or(false)
    }

    fn exclusive(&self) -> bool {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                QueueAttr::Exclusive(e) => Some(e.value),
                _ => None,
            })
            .next()
            .unwrap_or(false)
    }

    fn bindings(&self) -> QueueBindings {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                QueueAttr::Bindings(b) => Some(b.clone()),
                _ => None,
            })
            .next()
            .unwrap_or_else(QueueBindings::default)
    }
}

impl parse::Parse for QueueAttrsList {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let mut attrs = Vec::new();
        while input.is_empty() == false {
            attrs.push(input.parse()?);
        }
        Ok(QueueAttrsList(attrs))
    }
}

impl parse::Parse for QueueAttrs {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let ident = input.parse()?;
        let content;
        let _ = braced!(content in input);
        let attrs: Punctuated<_, Token![,]> = content.parse_terminated(QueueAttr::parse)?;
        Ok(QueueAttrs {
            ident,
            attrs: attrs.into_iter().collect(),
        })
    }
}

mod kw {
    custom_keyword!(name);
    custom_keyword!(with_priorities);
    custom_keyword!(exclusive);
    custom_keyword!(bindings);
}

impl parse::Parse for QueueAttr {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::name) {
            input.parse::<kw::name>()?;
            input.parse::<Token![=]>()?;
            Ok(QueueAttr::Name(input.parse()?))
        } else if lookahead.peek(kw::with_priorities) {
            input.parse::<kw::with_priorities>()?;
            input.parse::<Token![=]>()?;
            Ok(QueueAttr::WithPriorities(input.parse()?))
        } else if lookahead.peek(kw::exclusive) {
            input.parse::<kw::exclusive>()?;
            input.parse::<Token![=]>()?;
            Ok(QueueAttr::Exclusive(input.parse()?))
        } else if lookahead.peek(kw::bindings) {
            input.parse::<kw::bindings>()?;
            input.parse::<Token![=]>()?;
            Ok(QueueAttr::Bindings(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl parse::Parse for QueueBindings {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let content;
        let _ = braced!(content in input);
        let bindings: Punctuated<_, Token![,]> = content.parse_terminated(QueueBinding::parse)?;
        Ok(QueueBindings {
            bindings: bindings.into_iter().collect(),
        })
    }
}

impl parse::Parse for QueueBinding {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let exchange = input.parse()?;
        input.parse::<Token![=]>()?;
        let content;
        let _ = bracketed!(content in input);
        let jobs: Punctuated<_, Token![,]> = content.parse_terminated(syn::Path::parse)?;
        Ok(QueueBinding {
            exchange,
            jobs: jobs.into_iter().collect(),
        })
    }
}

impl ToTokens for QueueBindings {
    fn to_tokens(&self, dst: &mut TokenStream) {
        let output = self
            .bindings
            .iter()
            .fold(TokenStream::new(), |mut acc, el| {
                el.to_tokens(&mut acc);
                acc
            });
        dst.extend(output);
    }
}

impl ToTokens for QueueBinding {
    fn to_tokens(&self, dst: &mut TokenStream) {
        let exchange = &self.exchange;
        let mut output = quote!();
        for job in &self.jobs {
            output = quote! {
                #output
                .bind::<#exchange, #job>()
            };
        }
        dst.extend(output);
    }
}

impl Queue {
    fn new(attrs: QueueAttrs) -> Result<Self, Error> {
        const ERR_MISSING_NAME: &str = "missing mandatory name attribute";

        let queue = Queue {
            ident: attrs.ident.clone(),
            name: match attrs.name() {
                Some(name) => name.value(),
                None => return Err(Error::spanned(ERR_MISSING_NAME, attrs.ident.span())),
            },
            with_priorities: attrs.with_priorities(),
            exclusive: attrs.exclusive(),
            bindings: attrs.bindings(),
        };
        Ok(queue)
    }
}

impl ToTokens for Queue {
    fn to_tokens(&self, dst: &mut TokenStream) {
        let ident = &self.ident;
        let name = &self.name;
        let bindings = &self.bindings;
        let krate = quote!(::batch_rabbitmq);
        let export = quote!(#krate::export);

        let dummy_const = syn::Ident::new(
            &format!("__IMPL_BATCH_QUEUE_FOR_{}", ident.to_string()),
            Span::call_site(),
        );

        let output = quote! {
            pub struct #ident {
                inner: #krate::Queue
            }

            #[doc(hidden)]
            pub fn #ident(marker: #export::DeclareMarker) -> #ident {
                match marker {}
            }

            const #dummy_const: () = {
                impl #export::Declare for #ident {
                    const NAME: &'static str = #name;

                    type Input = #krate::QueueBuilder;

                    type Output = #krate::Queue;

                    type DeclareFuture = #export::Box<#export::Future<Item = Self, Error = #export::Error> + #export::Send>;

                    fn declare(declarator: &mut (impl #export::Declarator<Self::Input, Self::Output> + 'static)) -> Self::DeclareFuture {
                        use #export::Future;

                        let task = #krate::Queue::builder(Self::NAME.into())
                            // .with_priorities(true)
                            // .exclusive(true)
                            #bindings
                            .declare(declarator)
                            .map(|inner| #ident { inner });
                        #export::Box::new(task)
                    }
                }

                impl #export::Callbacks for #ident {
                    type Iterator = <#krate::Queue as #export::Callbacks>::Iterator;

                    fn callbacks(&self) -> Self::Iterator {
                        self.inner.callbacks()
                    }
                }
            };
        };
        dst.extend(output);
    }
}

fn do_parse(attrs: impl Iterator<Item = QueueAttrs>) -> Result<Vec<Queue>, Error> {
    let mut queues = Vec::new();
    for attr in attrs {
        queues.push(Queue::new(attr)?);
    }
    Ok(queues)
}

pub(crate) fn impl_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attrs = parse_macro_input!(input as QueueAttrsList);
    let queues = match do_parse(attrs.into_iter()) {
        Ok(queues) => queues,
        Err(e) => {
            return quote!( #e ).into();
        }
    };
    let mut output = quote!();
    for queue in queues.into_iter().map(|ex| ex.into_token_stream()) {
        output = quote! {
            #output
            #queue
        };
    }
    output.into()
}

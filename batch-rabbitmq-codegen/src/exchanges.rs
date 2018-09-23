use proc_macro;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn;
use syn::parse;
use syn::punctuated::Punctuated;

use error::Error;

struct ExchangeAttrsList(Vec<ExchangeAttrs>);

struct ExchangeAttrs {
    ident: syn::Ident,
    attrs: Vec<ExchangeAttr>,
}

enum ExchangeAttr {
    Name(syn::LitStr),
    Kind(ExchangeKind),
    Exclusive(syn::LitBool),
}

enum ExchangeKind {
    Direct,
    Fanout,
    Topic,
    Headers,
    Custom(syn::LitStr),
}

impl IntoIterator for ExchangeAttrsList {
    type Item = ExchangeAttrs;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ExchangeAttrs {
    fn name(&self) -> Option<&syn::LitStr> {
        self.attrs
            .iter()
            .filter_map(|a| match a {
                ExchangeAttr::Name(s) => Some(s),
                _ => None,
            })
            .next()
    }

    fn kind(&self) -> ExchangeKind {
        ExchangeKind::Direct
    }

    fn exclusive(&self) -> bool {
        false
    }
}

impl parse::Parse for ExchangeAttrsList {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let mut attrs = Vec::new();
        while input.is_empty() == false {
            attrs.push(input.parse()?);
        }
        Ok(ExchangeAttrsList(attrs))
    }
}

impl parse::Parse for ExchangeAttrs {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let ident = input.parse()?;
        let content;
        let _ = braced!(content in input);
        let attrs: Punctuated<_, Token![,]> = content.parse_terminated(ExchangeAttr::parse)?;
        Ok(ExchangeAttrs {
            ident,
            attrs: attrs.into_iter().collect(),
        })
    }
}

mod kw {
    custom_keyword!(name);
    custom_keyword!(kind);
    custom_keyword!(exclusive);
}

impl parse::Parse for ExchangeAttr {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::name) {
            input.parse::<kw::name>()?;
            input.parse::<Token![=]>()?;
            Ok(ExchangeAttr::Name(input.parse()?))
        } else if lookahead.peek(kw::kind) {
            input.parse::<kw::kind>()?;
            input.parse::<Token![=]>()?;
            Ok(ExchangeAttr::Kind(input.parse()?))
        } else if lookahead.peek(kw::exclusive) {
            input.parse::<kw::exclusive>()?;
            input.parse::<Token![=]>()?;
            Ok(ExchangeAttr::Exclusive(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

mod kinds {
    custom_keyword!(direct);
    custom_keyword!(fanout);
    custom_keyword!(topic);
    custom_keyword!(headers);
    custom_keyword!(custom);
}

impl parse::Parse for ExchangeKind {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kinds::direct) {
            input.parse::<kinds::direct>()?;
            Ok(ExchangeKind::Direct)
        } else if lookahead.peek(kinds::fanout) {
            input.parse::<kinds::fanout>()?;
            Ok(ExchangeKind::Fanout)
        } else if lookahead.peek(kinds::topic) {
            input.parse::<kinds::topic>()?;
            Ok(ExchangeKind::Topic)
        } else if lookahead.peek(kinds::headers) {
            input.parse::<kinds::headers>()?;
            Ok(ExchangeKind::Headers)
        } else if lookahead.peek(kinds::custom) {
            input.parse::<kinds::custom>()?;
            let content;
            let _ = braced!(content in input);
            Ok(ExchangeKind::Custom(content.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for ExchangeKind {
    fn to_tokens(&self, stream: &mut TokenStream) {
        let tokens = match self {
            ExchangeKind::Direct => quote!(::batch_rabbitmq::ExchangeKind::Direct),
            ExchangeKind::Fanout => quote!(::batch_rabbitmq::ExchangeKind::Fanout),
            ExchangeKind::Topic => quote!(::batch_rabbitmq::ExchangeKind::Topic),
            ExchangeKind::Headers => quote!(::batch_rabbitmq::ExchangeKind::Headers),
            ExchangeKind::Custom(kind) => {
                quote!(::batch_rabbitmq::ExchangeKind::Custom(#kind.into()))
            }
        };
        stream.extend(tokens);
    }
}

struct Exchange {
    ident: syn::Ident,
    name: String,
    kind: ExchangeKind,
    exclusive: bool,
}

impl Exchange {
    fn new(attrs: ExchangeAttrs) -> Result<Self, Error> {
        const ERR_MISSING_NAME: &str = "missing mandatory name attribute";

        let exchange = Exchange {
            ident: attrs.ident.clone(),
            name: match attrs.name() {
                Some(name) => name.value(),
                None => return Err(Error::spanned(ERR_MISSING_NAME, attrs.ident.span())),
            },
            kind: attrs.kind(),
            exclusive: attrs.exclusive(),
        };
        Ok(exchange)
    }
}

impl ToTokens for Exchange {
    fn to_tokens(&self, stream: &mut TokenStream) {
        let ident = &self.ident;
        let name = &self.name;
        let kind = &self.kind;
        let exclusive = &self.exclusive;
        let krate = quote!(::batch_rabbitmq);
        let export = quote!(#krate::export);

        let dummy_const = syn::Ident::new(
            &format!("__IMPL_BATCH_EXCHANGE_FOR_{}", ident.to_string()),
            Span::call_site(),
        );

        let output = quote! {
            pub struct #ident {
                inner: #krate::Exchange,
            }

            #[doc(hidden)]
            pub fn #ident(marker: #export::DeclareMarker) -> #ident {
                match marker {}
            }

            const #dummy_const: () = {
                impl #export::Declare for #ident {
                    const NAME: &'static str = #name;

                    type Input = #krate::ExchangeBuilder;

                    type Output = #krate::Exchange;

                    type DeclareFuture = #export::Box<#export::Future<Item = Self, Error = #export::Error> + #export::Send>;

                    fn declare(declarator: &mut (impl #export::Declarator<Self::Input, Self::Output> + 'static)) -> Self::DeclareFuture {
                        use #export::Future;

                        let task = #krate::Exchange::builder(Self::NAME.into())
                            .kind(#kind)
                            .exclusive(#exclusive)
                            .declare(declarator)
                            .map(|inner| #ident { inner });
                        Box::new(task)
                    }
                }

                impl<J> #export::With<J> for #ident
                where
                    J: #export::Job
                {
                    type Query = #krate::Query<J>;

                    fn with(&self, job: J) -> Self::Query {
                        self.inner.with(job)
                    }
                }
            };
        };
        stream.extend(output)
    }
}

fn do_parse(attrs: impl Iterator<Item = ExchangeAttrs>) -> Result<Vec<Exchange>, Error> {
    let mut exchanges = Vec::new();
    for attr in attrs {
        exchanges.push(Exchange::new(attr)?);
    }
    Ok(exchanges)
}

pub(crate) fn impl_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attrs = parse_macro_input!(input as ExchangeAttrsList);
    let exchanges = match do_parse(attrs.into_iter()) {
        Ok(exchanges) => exchanges,
        Err(e) => {
            return quote!( #e ).into();
        }
    };
    let mut output = quote!();
    for exchange in exchanges.into_iter().map(|ex| ex.into_token_stream()) {
        output = quote! {
            #output

            #exchange
        };
    }
    output.into()
}

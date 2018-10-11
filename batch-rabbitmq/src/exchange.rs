/// Allowed types for RabbitMQ's exchanges.
///
/// Built-in types are represented as dedicated enum variants, and you can specify other exchange types using the
/// `Kind::Custom` variant.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Kind {
    /// RabbitMQ's direct exchange type
    Direct,
    /// RabbitMQ's fanout exchange type
    Fanout,
    /// RabbitMQ's topic exchange type
    Topic,
    /// RabbitMQ's headers exchange type
    Headers,
    /// A custom exchange type
    Custom(String),
}

impl Default for Kind {
    /// The default exchange type is `direct`.
    fn default() -> Kind {
        Kind::Direct
    }
}

impl AsRef<str> for Kind {
    fn as_ref(&self) -> &str {
        match *self {
            Kind::Direct => "direct",
            Kind::Fanout => "fanout",
            Kind::Topic => "topic",
            Kind::Headers => "headers",
            Kind::Custom(ref name) => name.as_ref(),
        }
    }
}

/// A builder for the `Exchange` type.
///
/// You should not have to construct values of this type yourself, and should instead let
/// `batch_rabbitmq`'s procedural macros generate the necessary invocations.
#[derive(Debug, Clone)]
pub struct Builder {
    pub(crate) name: String,
    pub(crate) kind: Kind,
}

impl Builder {
    /// Sets the kind of the exchange that will be created.
    ///
    /// # Example
    ///
    /// ```
    /// use batch_rabbitmq::{Exchange, ExchangeKind};
    ///
    /// let kind = ExchangeKind::Topic;
    /// let builder = Exchange::build("batch-exchange")
    ///     .kind(kind);
    /// ```
    pub fn kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    /// Consume the builder and create the exchange.
    ///
    /// # Example
    ///
    /// ```
    /// use batch_rabbitmq::Exchange;
    ///
    /// let exchange = Exchange::build("batch-example").finish();
    /// ```
    pub fn finish(self) -> Exchange {
        Exchange {
            name: self.name,
            kind: self.kind,
        }
    }
}

/// A RabbitMQ exchange.
///
/// You should not have to construct values of this type yourself, and should instead let
/// `batch_rabbitmq`'s procedural macros generate the necessary invocations.
#[derive(Debug, Clone)]
pub struct Exchange {
    name: String,
    kind: Kind,
}

impl Exchange {
    /// Create a new `Exchange` with the default exchange type (direct).
    ///
    /// # Example
    ///
    /// ```
    /// use batch_rabbitmq::Exchange;
    ///
    /// let exchange = Exchange::new("batch-example");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Exchange {
            name: name.into(),
            kind: Kind::default(),
        }
    }

    /// Create a builder for a new `Exchange` named `name`.
    ///
    /// # Example
    ///
    /// ```
    /// use batch_rabbitmq::{Exchange};
    ///
    /// let builder = Exchange::build("batch-example");
    /// ```
    pub fn build(name: impl Into<String>) -> Builder {
        Builder {
            name: name.into(),
            kind: Kind::Direct,
        }
    }

    /// The name of this exchange.
    ///
    /// # Example
    ///
    /// ```
    /// use batch_rabbitmq::Exchange;
    ///
    /// let exchange = Exchange::new("batch-example");
    /// assert!(exchange.name() == "batch-example");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The type of this exchange.
    ///
    /// This method is named `kind` instead of `type` because it conflicts with the `type` keyword
    /// of the Rust language.
    ///
    /// # Example
    ///
    /// ```
    /// use batch_rabbitmq::{Exchange, ExchangeKind};
    ///
    /// let exchange = Exchange::new("batch-example");
    /// assert!(exchange.kind() == &ExchangeKind::Direct);
    ///
    /// let exchange = Exchange::build("batch-example")
    ///     .kind(ExchangeKind::Custom("x-delay-exchange".into()))
    ///     .finish();
    /// assert!(exchange.kind() == &ExchangeKind::Custom("x-delay-exchange".into()));
    /// ```
    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_exchange_kind_as_ref_str() {
        struct TestCase {
            kind: Kind,
            expected: &'static str,
        }

        let cases = vec![
            TestCase {
                kind: Kind::Direct,
                expected: "direct",
            },
            TestCase {
                kind: Kind::Fanout,
                expected: "fanout",
            },
            TestCase {
                kind: Kind::Topic,
                expected: "topic",
            },
            TestCase {
                kind: Kind::Headers,
                expected: "headers",
            },
            TestCase {
                kind: Kind::Custom("".into()),
                expected: "",
            },
            TestCase {
                kind: Kind::Custom("x-delay-exchange".into()),
                expected: "x-delay-exchange",
            },
            TestCase {
                kind: Kind::Custom("direct".into()),
                expected: "direct",
            },
        ];

        for case in cases {
            assert_eq!(case.kind.as_ref(), case.expected);
        }
    }
}

/// Allowed types for RabbitMQ's exchanges.
///
/// Built-in types are represented as dedicated enum variants, and you can specify other exchange types using the
/// `Kind::Custom` variant.
#[derive(Debug, Clone)]
pub enum Kind {
    Direct,
    Fanout,
    Topic,
    Headers,
    Custom(String),
}

impl Default for Kind {
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
#[derive(Debug, Clone)]
pub struct Builder {
    pub(crate) name: String,
    pub(crate) kind: Kind,
}

impl Builder {
    pub fn kind(&mut self, kind: Kind) -> &mut Self {
        self.kind = kind;
        self
    }

    pub fn finish(self) -> Exchange {
        Exchange {
            name: self.name,
            kind: self.kind,
        }
    }
}

/// A RabbitMQ exchange.
#[derive(Debug, Clone)]
pub struct Exchange {
    name: String,
    kind: Kind,
}

impl Exchange {
    pub fn new(name: String) -> Self {
        Exchange {
            name,
            kind: Kind::Direct,
        }
    }

    pub fn build(name: String) -> Builder {
        Builder {
            name,
            kind: Kind::Direct,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

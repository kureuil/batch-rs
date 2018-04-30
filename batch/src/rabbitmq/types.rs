use std::cmp;
use std::collections::BTreeSet;

use lapin::channel::{ExchangeDeclareOptions, QueueDeclareOptions};
use lapin::types::{AMQPValue, FieldTable};

/// A binding from a queue to an exchange, or from an exchange to an exchange.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binding {
    exchange: String,
    routing_key: String,
}

impl Binding {
    pub fn exchange(&self) -> &str {
        &self.exchange
    }

    pub fn routing_key(&self) -> &str {
        &self.routing_key
    }
}

/// A `RabbitMQ` exchange.
#[derive(Clone, Debug)]
pub struct Exchange {
    name: String,
    kind: String,
    bindings: BTreeSet<Binding>,
    options: ExchangeDeclareOptions,
    arguments: FieldTable,
}

impl Default for Exchange {
    fn default() -> Exchange {
        Exchange {
            name: "".into(),
            kind: "direct".into(),
            bindings: BTreeSet::new(),
            options: ExchangeDeclareOptions::default(),
            arguments: FieldTable::new(),
        }
    }
}

impl cmp::PartialEq for Exchange {
    fn eq(&self, other: &Exchange) -> bool {
        self.name == other.name
    }
}

impl cmp::Eq for Exchange {}

impl cmp::PartialOrd for Exchange {
    fn partial_cmp(&self, other: &Exchange) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Exchange {
    fn cmp(&self, other: &Exchange) -> cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Exchange {
    /// Return the name of this `Exchange`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the kind of this `Exchange`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Return the bindings associated to this `Exchange`.
    pub(crate) fn bindings(&self) -> &BTreeSet<Binding> {
        &self.bindings
    }

    /// Return the options of this `Exchange`.
    pub fn options(&self) -> &ExchangeDeclareOptions {
        &self.options
    }

    /// Return the arguments of this `Exchange`.
    pub fn arguments(&self) -> &FieldTable {
        &self.arguments
    }
}

/// A builder for `RabbitMQ` `Exchange`.
#[derive(Debug)]
pub struct ExchangeBuilder {
    name: String,
    bindings: BTreeSet<Binding>,
    options: ExchangeDeclareOptions,
    arguments: FieldTable,
}

impl ExchangeBuilder {
    /// Create a new `ExchangeBuilder` instance from the desired exchange name.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let builder = ExchangeBuilder::new("batch.example");
    /// ```
    pub fn new(name: &str) -> ExchangeBuilder {
        ExchangeBuilder {
            name: name.into(),
            bindings: BTreeSet::new(),
            options: ExchangeDeclareOptions::default(),
            arguments: FieldTable::new(),
        }
    }

    /// Binds this exchange to another exchange via a routing key.
    ///
    /// All of the messages posted to this exchange associated to the given routing key
    /// are automatically sent to the given exchange.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let builder = ExchangeBuilder::new("batch.example")
    ///     .bind("batch.messaging", "hello-world");
    /// ```
    pub fn bind(mut self, exchange: &str, routing_key: &str) -> Self {
        self.bindings.insert(Binding {
            exchange: exchange.into(),
            routing_key: routing_key.into(),
        });
        self
    }

    /// Return a reference the declare options for this exchange.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let builder = ExchangeBuilder::new("batch.example");
    /// {
    ///     let options = builder.options();
    ///     println!("Options: {:?}", options);
    /// }
    /// ```
    pub fn options(&self) -> &ExchangeDeclareOptions {
        &self.options
    }

    /// Return a mutable reference to the declare options for this exchange.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let mut builder = ExchangeBuilder::new("batch.example");
    /// {
    ///     let options = builder.options_mut();
    ///     options.durable = true;
    ///     println!("Options: {:?}", options);
    /// }
    /// ```
    pub fn options_mut(&mut self) -> &mut ExchangeDeclareOptions {
        &mut self.options
    }

    /// Return a reference to the exchange arguments.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let builder = ExchangeBuilder::new("batch.example");
    /// {
    ///     let arguments = builder.arguments();
    ///     println!("Arguments: {:?}", arguments);
    /// }
    /// ```
    pub fn arguments(&self) -> &FieldTable {
        &self.arguments
    }

    /// Return a mutable reference to the exchange arguments.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate batch;
    /// extern crate lapin_futures;
    ///
    /// use lapin_futures::types::AMQPValue;
    /// use batch::ExchangeBuilder;
    ///
    /// # fn main() {
    /// let mut builder = ExchangeBuilder::new("batch.example");
    /// {
    ///     let arguments = builder.arguments_mut();
    ///     arguments.insert("x-custom-argument".to_string(), AMQPValue::Boolean(true));
    ///     println!("Arguments: {:?}", arguments);
    /// }
    /// # }
    /// ```
    pub fn arguments_mut(&mut self) -> &mut FieldTable {
        &mut self.arguments
    }

    /// Set the durability. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let mut builder = ExchangeBuilder::new("batch.example")
    ///     .durable(true);
    ///
    /// 
    /// ```
    pub fn durable(mut self, durable: bool) -> Self {
        self.options.durable = durable;
        self
    }

    /// Set the auto_delete option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let mut builder = ExchangeBuilder::new("batch.example")
    ///     .auto_delete(true);
    ///
    /// 
    /// ```
    pub fn auto_delete(mut self, auto_delete: bool) -> Self {
        self.options.auto_delete = auto_delete;
        self
    }

    /// Set the nowait option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let mut builder = ExchangeBuilder::new("batch.example")
    ///     .nowait(true);
    ///
    /// 
    /// ```
    pub fn nowait(mut self, nowait: bool) -> Self {
        self.options.nowait = nowait;
        self
    }

    /// Set the internal option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let mut builder = ExchangeBuilder::new("batch.example")
    ///     .internal(true);
    ///
    /// 
    /// ```
    pub fn internal(mut self, internal: bool) -> Self {
        self.options.internal = internal;
        self
    }

    /// Set the passive option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let mut builder = ExchangeBuilder::new("batch.example")
    ///     .passive(true);
    ///
    /// 
    /// ```
    pub fn passive(mut self, passive: bool) -> Self {
        self.options.passive = passive;
        self
    }

    /// Set the ticket option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::ExchangeBuilder;
    ///
    /// let mut builder = ExchangeBuilder::new("batch.example")
    ///     .ticket(2);
    ///
    /// 
    /// ```
    pub fn ticket(mut self, ticket: u16) -> Self {
        self.options.ticket = ticket;
        self
    }

    /// Build a new `Exchange` instance from this builder data.
    pub(crate) fn build(self) -> Exchange {
        Exchange {
            name: self.name,
            kind: "direct".into(),
            bindings: self.bindings,
            options: self.options,
            arguments: self.arguments,
        }
    }
}

/// Shorthand to create a new `ExchangeBuilder` instance.
pub fn exchange(name: &str) -> ExchangeBuilder {
    ExchangeBuilder::new(name)
}

/// A `RabbitMQ` queue.
#[derive(Clone, Debug)]
pub struct Queue {
    name: String,
    bindings: BTreeSet<Binding>,
    options: QueueDeclareOptions,
    arguments: FieldTable,
}

impl cmp::PartialEq for Queue {
    fn eq(&self, other: &Queue) -> bool {
        self.name == other.name
    }
}

impl cmp::Eq for Queue {}

impl cmp::PartialOrd for Queue {
    fn partial_cmp(&self, other: &Queue) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Queue {
    fn cmp(&self, other: &Queue) -> cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Queue {
    /// Return the name of this `Queue`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the bindings associated to this `Queue`.
    pub(crate) fn bindings(&self) -> &BTreeSet<Binding> {
        &self.bindings
    }

    /// Return the options used when declaring this `Queue`.
    pub fn options(&self) -> &QueueDeclareOptions {
        &self.options
    }

    /// Return the arguments used when declaring this `Queue`.
    pub fn arguments(&self) -> &FieldTable {
        &self.arguments
    }
}

/// A builder for `RabbitMQ` `Queue`.
#[derive(Debug)]
pub struct QueueBuilder {
    name: String,
    bindings: BTreeSet<Binding>,
    options: QueueDeclareOptions,
    arguments: FieldTable,
}

impl QueueBuilder {
    /// Create a new `QueueBuilder` from the desired queue name.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let queue = QueueBuilder::new("video-transcoding");
    /// ```
    pub fn new(name: &str) -> QueueBuilder {
        QueueBuilder {
            name: name.into(),
            bindings: BTreeSet::new(),
            options: QueueDeclareOptions::default(),
            arguments: FieldTable::new(),
        }
    }

    /// Return a reference the declare options for this queue.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let builder = QueueBuilder::new("video-transcoding");
    /// {
    ///     let options = builder.options();
    ///     println!("Options: {:?}", options);
    /// }
    /// ```
    pub fn options(&self) -> &QueueDeclareOptions {
        &self.options
    }

    /// Return a mutable reference the declare options for this queue.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let mut builder = QueueBuilder::new("video-transcoding");
    /// {
    ///     let options = builder.options_mut();
    ///     options.auto_delete = true;
    ///     println!("Options: {:?}", options);
    /// }
    /// ```
    pub fn options_mut(&mut self) -> &mut QueueDeclareOptions {
        &mut self.options
    }

    /// Return a reference to the queue arguments.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let builder = QueueBuilder::new("video-transcoding");
    /// {
    ///     let arguments = builder.arguments();
    ///     println!("Arguments: {:?}", arguments);
    /// }
    /// ```
    pub fn arguments(&self) -> &FieldTable {
        &self.arguments
    }

    /// Return a mutable reference to the queue arguments.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate batch;
    /// extern crate lapin_futures;
    ///
    /// use lapin_futures::types::AMQPValue;
    /// use batch::QueueBuilder;
    ///
    /// # fn main() {
    /// let mut builder = QueueBuilder::new("video-transcoding");
    /// {
    ///     let arguments = builder.arguments_mut();
    ///     arguments.insert("x-custom-argument".to_string(), AMQPValue::Boolean(true));
    ///     println!("Arguments: {:?}", arguments);
    /// }
    /// # }
    /// ```
    pub fn arguments_mut(&mut self) -> &mut FieldTable {
        &mut self.arguments
    }

    /// Bind this queue to an exchange via a routing key.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// QueueBuilder::new("video-transcoding")
    ///     .bind("movies", "transcoding")
    ///     .bind("series", "transcoding")
    ///     .bind("anime", "transcoding");
    /// ```
    pub fn bind(mut self, exchange: &str, routing_key: &str) -> Self {
        self.bindings.insert(Binding {
            exchange: exchange.into(),
            routing_key: routing_key.into(),
        });
        self
    }

    /// Set the durability. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let mut builder = QueueBuilder::new("video-transcoding")
    ///     .durable(true);
    /// 
    /// 
    /// ```
    pub fn durable(mut self, durable: bool) -> Self {
        self.options.durable = durable;
        self
    }

    /// Set the auto_delete option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let mut builder = QueueBuilder::new("video-transcoding")
    ///     .auto_delete(true);
    /// 
    /// 
    /// ```
    pub fn auto_delete(mut self, auto_delete: bool) -> Self {
        self.options.auto_delete = auto_delete;
        self
    }

    /// Set the nowait option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let mut builder = QueueBuilder::new("video-transcoding")
    ///     .nowait(true);
    /// 
    /// 
    /// ```
    pub fn nowait(mut self, nowait: bool) -> Self {
        self.options.nowait = nowait;
        self
    }

    /// Set the exclusive option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let mut builder = QueueBuilder::new("video-transcoding")
    ///     .exclusive(true);
    /// 
    /// 
    /// ```
    pub fn exclusive(mut self, exclusive: bool) -> Self {
        self.options.exclusive = exclusive;
        self
    }

    /// Set the passive option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let mut builder = QueueBuilder::new("video-transcoding")
    ///     .passive(true);
    /// 
    /// 
    /// ```
    pub fn passive(mut self, passive: bool) -> Self {
        self.options.passive = passive;
        self
    }

    /// Set the ticket option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// let mut builder = QueueBuilder::new("video-transcoding")
    ///     .ticket(2);
    /// 
    /// 
    /// ```
    pub fn ticket(mut self, ticket: u16) -> Self {
        self.options.ticket = ticket;
        self
    } 


    /// Enable task priorities on this queue.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::QueueBuilder;
    ///
    /// QueueBuilder::new("video-transcoding")
    ///     .enable_priorities();
    /// ```
    pub fn enable_priorities(mut self) -> Self {
        self.arguments
            .insert("x-max-priority".to_string(), AMQPValue::ShortShortUInt(4));
        self
    }

    /// Create a new `Queue` instance from this builder data.
    pub(crate) fn build(self) -> Queue {
        Queue {
            name: self.name,
            bindings: self.bindings,
            options: self.options,
            arguments: self.arguments,
        }
    }
}

/// Shorthand to create a new `QueueBuilder` instance.
pub fn queue(name: &str) -> QueueBuilder {
    QueueBuilder::new(name)
}

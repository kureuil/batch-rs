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
    /// Create a new `ExchangeBuilder` instance from the desired exchange name.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::Exchange;
    ///
    /// let builder = Exchange::builder("batch.example");
    /// ```
    pub fn builder(name: &str) -> ExchangeBuilder {
        ExchangeBuilder::new(name)
    }

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
///
/// See [`Exchange::builder`](struct.Exchange.html#method.builder).
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
    /// use batch::Exchange;
    ///
    /// let builder = Exchange::builder("batch.example");
    /// ```
    fn new(name: &str) -> ExchangeBuilder {
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
    /// use batch::Exchange;
    ///
    /// let builder = Exchange::builder("batch.example")
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
    /// use batch::Exchange;
    ///
    /// let builder = Exchange::builder("batch.example");
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
    /// use batch::Exchange;
    ///
    /// let mut builder = Exchange::builder("batch.example");
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
    /// use batch::Exchange;
    ///
    /// let builder = Exchange::builder("batch.example");
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
    /// use batch::Exchange;
    ///
    /// # fn main() {
    /// let mut builder = Exchange::builder("batch.example");
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

    /// Set the durable option. Chainable.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::Exchange;
    ///
    /// let mut builder = Exchange::builder("batch.example")
    ///     .durable(true);
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
    /// use batch::Exchange;
    ///
    /// let mut builder = Exchange::builder("batch.example")
    ///     .auto_delete(true);
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
    /// use batch::Exchange;
    ///
    /// let mut builder = Exchange::builder("batch.example")
    ///     .nowait(true);
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
    /// use batch::Exchange;
    ///
    /// let mut builder = Exchange::builder("batch.example")
    ///     .internal(true);
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
    /// use batch::Exchange;
    ///
    /// let mut builder = Exchange::builder("batch.example")
    ///     .passive(true);
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
    /// use batch::Exchange;
    ///
    /// let mut builder = Exchange::builder("batch.example")
    ///     .ticket(2);
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
    /// Create a new `QueueBuilder` from the desired queue name.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::Queue;
    ///
    /// let queue = Queue::builder("video-transcoding");
    /// ```
    pub fn builder(name: &str) -> QueueBuilder {
        QueueBuilder::new(name)
    }

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
///
/// See [`Queue::builder`](struct.Queue.html#method.builder).
#[derive(Debug)]
pub struct QueueBuilder {
    name: String,
    bindings: BTreeSet<Binding>,
    options: QueueDeclareOptions,
    arguments: FieldTable,
}

impl QueueBuilder {
    fn new(name: &str) -> QueueBuilder {
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
    /// use batch::Queue;
    ///
    /// let builder = Queue::builder("video-transcoding");
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
    /// use batch::Queue;
    ///
    /// let mut builder = Queue::builder("video-transcoding");
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
    /// use batch::Queue;
    ///
    /// let builder = Queue::builder("video-transcoding");
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
    /// use batch::Queue;
    ///
    /// # fn main() {
    /// let mut builder = Queue::builder("video-transcoding");
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
    /// use batch::Queue;
    ///
    /// Queue::builder("video-transcoding")
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
    /// use batch::Queue;
    ///
    /// let mut builder = Queue::builder("video-transcoding")
    ///     .durable(true);
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
    /// use batch::Queue;
    ///
    /// let mut builder = Queue::builder("video-transcoding")
    ///     .auto_delete(true);
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
    /// use batch::Queue;
    ///
    /// let mut builder = Queue::builder("video-transcoding")
    ///     .nowait(true);
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
    /// use batch::Queue;
    ///
    /// let mut builder = Queue::builder("video-transcoding")
    ///     .exclusive(true);
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
    /// use batch::Queue;
    ///
    /// let mut builder = Queue::builder("video-transcoding")
    ///     .passive(true);
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
    /// use batch::Queue;
    ///
    /// let mut builder = Queue::builder("video-transcoding")
    ///     .ticket(2);
    /// ```
    pub fn ticket(mut self, ticket: u16) -> Self {
        self.options.ticket = ticket;
        self
    }

    /// Enable priorities on this queue.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::Queue;
    ///
    /// Queue::builder("video-transcoding")
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

use crate::ConnectionBuilder;

/// A trait used to declare queues to RabbitMQ.
pub trait Declare {
    /// Declare the current type to the given [`ConnectionBuilder`].
    fn declare(conn: &mut ConnectionBuilder);
}

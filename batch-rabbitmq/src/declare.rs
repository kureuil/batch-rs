use ConnectionBuilder;

/// A trait used to declare queues to RabbitMQ.
pub trait Declare {
    fn declare(conn: &mut ConnectionBuilder);
}

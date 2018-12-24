use std::fmt;

/// A job retrieved from a Redis server, waiting to be performed.
pub struct Delivery {}

impl fmt::Debug for Delivery {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Delivery").finish()
    }
}

impl batch::Delivery for Delivery {
    type AckFuture = Box<futures::Future<Item = (), Error = failure::Error> + Send>;

    type RejectFuture = Box<futures::Future<Item = (), Error = failure::Error> + Send>;

    fn payload(&self) -> &[u8] {
        unimplemented!();
    }

    fn properties(&self) -> &batch::Properties {
        unimplemented!();
    }

    fn ack(self) -> Self::AckFuture {
        unimplemented!();
    }

    fn reject(self) -> Self::RejectFuture {
        unimplemented!();
    }
}

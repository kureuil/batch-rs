use std::fmt;

use crate::delivery::Delivery;

/// Retrieves jobs from a Redis server.
pub struct Consumer {}

impl fmt::Debug for Consumer {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Consumer")
			.finish()
	}
}

impl futures::Stream for Consumer {
	type Item = Delivery;

	type Error = failure::Error;

	fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
		unimplemented!()
	}
}

impl batch::Consumer for Consumer {
	type Delivery = Delivery;
}

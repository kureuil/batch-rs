use failure::Error;
use futures::{future, Future};

use { Properties };

#[derive(Debug)]
pub struct Query<J> {
    job: J,
    exchange: String,
    properties: Properties,
}

impl<J> Query<J>
where
    J: ::Job,
{
    pub(crate) fn new(job: J) -> Self {
        Query {
            job,
            exchange: String::from(""),
            properties: Properties::new(J::NAME),
        }
    }

    pub fn priority(mut self, priority: ::Priority) -> Self {
        self.properties.priority = priority;
        self
    }

    pub fn perform_later(self, client: &Query<J>) -> impl Future<Item = (), Error = Error> {
        future::empty()
    }
}

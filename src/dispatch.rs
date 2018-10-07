use failure::Error;
use serde::Serialize;
use serde_json::to_vec;

use {Job, Properties};

/// A message that can be sent to a broker.
#[derive(Debug, Serialize)]
pub struct Dispatch {
    destination: String,
    payload: Vec<u8>,
    properties: Properties,
}

impl Dispatch {
    pub(crate) fn new(
        destination: String,
        job: impl Job,
        properties: Properties,
    ) -> Result<Self, Error> {
        Ok(Dispatch {
            payload: to_vec(&job).map_err(|e| Error::from(e))?,
            destination,
            properties,
        })
    }

    /// An indicator on where should this message be dispatched.
    pub fn destination(&self) -> &str {
        &self.destination
    }

    /// Get a reference to the serialized `Job` instance associated to this delivery.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Get a reference to the `Properties` instance associated to this delivery.
    pub fn properties(&self) -> &Properties {
        &self.properties
    }
}

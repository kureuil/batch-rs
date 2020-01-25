use std::error::Error;
use serde_json::to_vec;

use crate::{Job, Properties};

/// A message that will be sent to a broker.
///
/// A dispatch stores a serialized job, its properties and the key used to publish
/// it (e.g: the name of the queue it willbe published to).
///
/// Instances of dispatch are internally created when you dispatch a `Query`, so you only need
/// to know about them if you're writing an adapter for Batch.
#[derive(Debug)]
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
    ) -> Result<Self, Box<dyn Error + Send>> {
        Ok(Dispatch {
            payload: to_vec(&job).map_err(|e| crate::QuickError::boxed(e))?,
            destination,
            properties,
        })
    }

    /// An indicator on where should this message be dispatched, typically the queue's name.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::Dispatch;
    ///
    /// fn example(dispatch: Dispatch) {
    ///     println!("dispatch destination: {:?}", dispatch.destination());
    /// }
    /// ```
    pub fn destination(&self) -> &str {
        &self.destination
    }

    /// Get a reference to the serialized `Job` instance associated to this delivery.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::Dispatch;
    ///
    /// fn example(dispatch: Dispatch) {
    ///     println!("dispatch payload: {:?}", dispatch.payload());
    /// }
    /// ```
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Get a reference to the `Properties` instance associated to this delivery.
    ///
    /// # Example
    ///
    /// ```
    /// use batch::Dispatch;
    ///
    /// fn example(dispatch: Dispatch) {
    ///     println!("dispatch properties: {:?}", dispatch.properties());
    /// }
    /// ```
    pub fn properties(&self) -> &Properties {
        &self.properties
    }
}

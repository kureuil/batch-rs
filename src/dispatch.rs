use job::Properties;

/// A message that can be sent to a broker.
pub trait Dispatch {
    /// Get a reference to the serialized `Job` instance associated to this delivery.
    fn payload(&self) -> &[u8];

    /// Get a reference to the `Properties` instance associated to this delivery.
    fn properties(&self) -> &Properties;
}

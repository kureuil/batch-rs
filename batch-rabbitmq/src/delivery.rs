use batch;
use failure::Error;
use futures::{Future, Poll};
use lapin::channel;
use lapin::message;
use lapin::types::AMQPValue;
use log::warn;
use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;
use uuid::Uuid;

use stream;

/// A delivery is a message received from RabbitMQ.
///
/// Deliveries must be either acked or rejected before being dropped to signal RabbitMQ of the
/// status of the associated job. If RabbitMQ doesn't get a response once the Time To Live of
/// the delivery has expired, the delivery is marked as rejected and set to be retried. You can do
/// so using the methods provided by the [`batch::Delivery`] trait.
pub struct Delivery {
    payload: Vec<u8>,
    delivery_tag: u64,
    properties: batch::Properties,
    channel: channel::Channel<stream::Stream>,
}

impl fmt::Debug for Delivery {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Delivery")
            .field("payload", &self.payload)
            .field("delivery_tag", &self.delivery_tag)
            .field("properties", &self.properties)
            .finish()
    }
}

impl Delivery {
    pub(crate) fn new(
        delivery: message::Delivery,
        channel: channel::Channel<stream::Stream>,
    ) -> Result<Self, Error> {
        let payload = delivery.data;
        let delivery_tag = delivery.delivery_tag;
        let properties = {
            let empty = BTreeMap::new();
            let headers = delivery.properties.headers().as_ref().unwrap_or(&empty);
            let task = match headers.get("task") {
                Some(AMQPValue::LongString(v)) => v,
                Some(_) => bail!("Incorrect type for `task` header from delivery"),
                None => bail!("Missing `task` header from delivery"),
            };
            let mut props = batch::Properties::new(task);
            props.id = match delivery.properties.correlation_id() {
                Some(raw_id) => Uuid::parse_str(&raw_id).map_err(Error::from)?,
                None => bail!("Missing `correlation_id` property from delivery"),
            };
            props.content_type = match delivery.properties.content_type().as_ref() {
                Some(v) if v == "application/json" => v.to_string(),
                Some(_) => bail!("Invalid value for `content_type` property from delivery"),
                None => bail!("Missing `content_type` property from delivery"),
            };
            props.content_encoding = match delivery.properties.content_encoding().as_ref() {
                Some(v) if v == "utf-8" => v.to_string(),
                Some(_) => bail!("Invalid value for `content_encoding` property from delivery"),
                None => bail!("Missing `content_encoding` property from delivery"),
            };
            props.lang = match headers.get("lang") {
                Some(AMQPValue::LongString(v)) => v.to_string(),
                Some(_) => bail!("Incorrect type for `lang` header from delivery"),
                None => bail!("Missing `lang` header from delivery"),
            };
            props.root_id = match headers.get("root_id") {
                Some(AMQPValue::LongString(raw_id)) => {
                    Some(Uuid::parse_str(&raw_id).map_err(Error::from)?)
                }
                Some(AMQPValue::Void) => None,
                Some(_) => bail!("Incorrect type for `root_id` header from delivery"),
                None => bail!("Missing `root_id` header from delivery"),
            };
            props.parent_id = match headers.get("parent_id") {
                Some(AMQPValue::LongString(raw_id)) => {
                    Some(Uuid::parse_str(&raw_id).map_err(Error::from)?)
                }
                Some(AMQPValue::Void) => None,
                Some(_) => bail!("Incorrect type for `parent_id` header from delivery"),
                None => bail!("Missing `parent_id` header from delivery"),
            };
            props.group = match headers.get("group") {
                Some(AMQPValue::LongString(raw_id)) => {
                    Some(Uuid::parse_str(&raw_id).map_err(Error::from)?)
                }
                Some(AMQPValue::Void) => None,
                Some(_) => bail!("Incorrect type for `group` header from delivery"),
                None => bail!("Missing `group` header from delivery"),
            };
            props.timelimit = match headers.get("timelimit") {
                Some(AMQPValue::FieldArray(array)) => match array.as_slice() {
                    [softlimit, hardlimit] => {
                        let softlimit = match softlimit {
							AMQPValue::Void => None,
							AMQPValue::Timestamp(ms) => Some(Duration::from_millis(*ms)),
							_ => bail!("Incorrect type for first value of `timelimit` header from delivery"),
						};
                        let hardlimit = match hardlimit {
							AMQPValue::Void => None,
							AMQPValue::Timestamp(ms) => Some(Duration::from_millis(*ms)),
							_ => bail!("Incorrect type for second value of `timelimit` header from delivery"),
						};
                        if hardlimit < softlimit {
                            warn!(
                                "hardlimit is less than softlimit, replacing with softlimit; id={} job={} delivery_tag={}",
                                props.id,
                                props.task,
                                delivery_tag
                            );
                            (softlimit, softlimit)
                        } else {
                            (softlimit, hardlimit)
                        }
                    }
                    _ => bail!("Invalid value for `timelimit` header from delivery"),
                },
                Some(_) => bail!("Incorrect type for `timelimit` header from delivery"),
                None => (None, None),
            };
            props
        };
        Ok(Delivery {
            payload,
            delivery_tag,
            properties,
            channel,
        })
    }
}

/// Future returned when acking a [`Delivery`] with [`batch::Delivery::ack`].
#[must_use = "futures do nothing unless polled"]
pub struct AcknowledgeFuture(Box<Future<Item = (), Error = Error> + Send>);

impl fmt::Debug for AcknowledgeFuture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AcknowledgeFuture").finish()
    }
}

impl Future for AcknowledgeFuture {
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

/// Future returned when rejecting a [`Delivery`] with [`batch::Delivery::reject`].
#[must_use = "futures do nothing unless polled"]
pub struct RejectFuture(Box<Future<Item = (), Error = Error> + Send>);

impl fmt::Debug for RejectFuture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RejectFuture").finish()
    }
}

impl Future for RejectFuture {
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl batch::Delivery for Delivery {
    type AckFuture = AcknowledgeFuture;

    type RejectFuture = RejectFuture;

    fn payload(&self) -> &[u8] {
        &self.payload
    }

    fn properties(&self) -> &batch::Properties {
        &self.properties
    }

    fn ack(self) -> Self::AckFuture {
        let id = self.properties().id;
        let job = &self.properties().task;
        let delivery_tag = self.delivery_tag;
        debug!(
            "ack; id={} job={:?} delivery_tag={:?}",
            id, job, delivery_tag
        );
        let task = self
            .channel
            .basic_ack(self.delivery_tag, false)
            .map_err(Error::from);
        AcknowledgeFuture(Box::new(task))
    }

    fn reject(self) -> Self::RejectFuture {
        let id = self.properties().id;
        let job = &self.properties().task;
        let delivery_tag = self.delivery_tag;
        debug!(
            "reject; id={} job={:?} delivery_tag={:?}",
            id, job, delivery_tag
        );
        let task = self
            .channel
            .basic_reject(self.delivery_tag, false)
            .map_err(Error::from);
        RejectFuture(Box::new(task))
    }
}

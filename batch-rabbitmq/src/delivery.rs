use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use batch;
use failure::Error;
use futures::sync::mpsc;
use futures::{Future, Poll, Sink};
use lapin::message;
use lapin::types::AMQPValue;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Completion {
    Acknowledge(u64),
    Reject(u64),
}

#[derive(Debug)]
pub struct Delivery {
    payload: Vec<u8>,
    delivery_tag: u64,
    properties: batch::Properties,
    channel: mpsc::Sender<Completion>,
}

impl Delivery {
    pub fn new(
        delivery: message::Delivery,
        channel: mpsc::Sender<Completion>,
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
                        // TODO: check that hardlimit if greater than or equal to the softlimit
                        (softlimit, hardlimit)
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

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn properties(&self) -> &batch::Properties {
        &self.properties
    }

    pub fn ack(self) -> Acknowledge {
        let task = self
            .channel
            .send(Completion::Acknowledge(self.delivery_tag))
            .map(|_| ())
            .map_err(Error::from);
        Acknowledge(Box::new(task))
    }

    pub fn reject(self) -> Reject {
        let task = self
            .channel
            .send(Completion::Reject(self.delivery_tag))
            .map(|_| ())
            .map_err(Error::from);
        Reject(Box::new(task))
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct Acknowledge(Box<Future<Item = (), Error = Error> + Send>);

impl fmt::Debug for Acknowledge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Acknowledge").finish()
    }
}

impl Future for Acknowledge {
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct Reject(Box<Future<Item = (), Error = Error> + Send>);

impl fmt::Debug for Reject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Reject").finish()
    }
}

impl Future for Reject {
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl batch::Delivery for Delivery {
    type AckFuture = Acknowledge;

    type RejectFuture = Reject;

    fn payload(&self) -> &[u8] {
        self.payload()
    }

    fn properties(&self) -> &batch::Properties {
        self.properties()
    }

    fn ack(self) -> Self::AckFuture {
        self.ack()
    }

    fn reject(self) -> Self::RejectFuture {
        self.reject()
    }
}

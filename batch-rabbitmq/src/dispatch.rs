use batch_core as batch;
use failure::Error;
use lapin::channel::BasicProperties;
use lapin::types::{AMQPValue, FieldTable};
use serde::{Serialize, Serializer};
use serde_json::to_vec;

#[derive(Debug, Serialize)]
pub struct Dispatch {
    exchange: String,
    payload: Vec<u8>,
    properties: batch::Properties,
}

impl Dispatch {
    pub(crate) fn new(
        exchange: String,
        job: impl batch::Job,
        properties: batch::Properties,
    ) -> Result<Self, Error> {
        struct Ser<J: batch::Job>(J);

        impl<J: batch::Job> Serialize for Ser<J> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.serialize(serializer)
            }
        }

        let ser = Ser(job);
        Ok(Dispatch {
            payload: to_vec(&ser).map_err(|e| Error::from(e))?,
            exchange,
            properties,
        })
    }

    pub fn exchange(&self) -> &str {
        &self.exchange
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn properties(&self) -> &batch::Properties {
        &self.properties
    }

    pub fn to_amqp_properties(&self) -> BasicProperties {
        let mut headers = FieldTable::new();
        headers.insert("lang".into(), AMQPValue::LongString("rs".into()));
        headers.insert(
            "task".into(),
            AMQPValue::LongString(self.properties.task.clone()),
        );
        headers.insert("root_id".into(), AMQPValue::Void);
        headers.insert("parent_id".into(), AMQPValue::Void);
        headers.insert("group".into(), AMQPValue::Void);
        BasicProperties::default()
            .with_content_type("application/json".to_string())
            .with_content_encoding("utf-8".to_string())
            .with_priority(self.properties.priority as u8)
            .with_correlation_id(self.properties.id.hyphenated().to_string())
            .with_headers(headers)
    }
}

impl batch::Dispatch for Dispatch {
    fn payload(&self) -> &[u8] {
        self.payload()
    }

    fn properties(&self) -> &batch::Properties {
        self.properties()
    }
}

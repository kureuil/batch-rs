use std::time::Duration;

use lapin::types;
use lapin_async::generated::basic::Properties;
use lapin_async::queue::Message;
use lapin_async::types::{AMQPValue, FieldTable};

#[derive(Serialize, Deserialize)]
#[serde(remote = "Properties")]
struct PropertiesDef {
    pub content_type: Option<types::ShortString>,
    pub content_encoding: Option<types::ShortString>,
    pub headers: Option<types::FieldTable>,
    pub delivery_mode: Option<types::ShortShortUInt>,
    pub priority: Option<types::ShortShortUInt>,
    pub correlation_id: Option<types::ShortString>,
    pub reply_to: Option<types::ShortString>,
    pub expiration: Option<types::ShortString>,
    pub message_id: Option<types::ShortString>,
    pub timestamp: Option<types::Timestamp>,
    pub type_: Option<types::ShortString>,
    pub user_id: Option<types::ShortString>,
    pub app_id: Option<types::ShortString>,
    pub cluster_id: Option<types::ShortString>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Message")]
struct MessageDef {
    pub delivery_tag: types::LongLongUInt,
    pub exchange: String,
    pub routing_key: String,
    pub redelivered: bool,
    #[serde(with = "PropertiesDef")]
    pub properties: Properties,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delivery(#[serde(with = "MessageDef")] pub Message);

impl Delivery {
    pub fn tag(&self) -> u64 {
        self.0.delivery_tag
    }

    pub fn task(&self) -> &str {
        self.0
            .properties
            .headers
            .as_ref()
            .map(|hdrs| match hdrs.get("task") {
                Some(&AMQPValue::LongString(ref task)) => task.as_ref(),
                _ => "",
            })
            .unwrap_or("")
    }

    pub fn task_id(&self) -> &str {
        self.0
            .properties
            .correlation_id
            .as_ref()
            .map_or("", String::as_ref)
    }

    pub fn exchange(&self) -> &str {
        &self.0.exchange
    }

    pub fn routing_key(&self) -> &str {
        &self.0.routing_key
    }

    pub fn data(&self) -> &[u8] {
        &self.0.data
    }

    pub fn properties(&self) -> &Properties {
        &self.0.properties
    }

    pub fn timeout(&self) -> (Option<Duration>, Option<Duration>) {
        self.0
            .properties
            .headers
            .as_ref()
            .map(|hdrs| match hdrs.get("timelimit") {
                Some(&AMQPValue::FieldArray(ref vec)) if vec.len() == 2 => {
                    let soft_limit = match vec[0] {
                        AMQPValue::Timestamp(s) => Some(Duration::from_secs(s)),
                        _ => None,
                    };
                    let hard_limit = match vec[1] {
                        AMQPValue::Timestamp(s) => Some(Duration::from_secs(s)),
                        _ => None,
                    };
                    (soft_limit, hard_limit)
                }
                _ => (None, None),
            })
            .unwrap_or((None, None))
    }

    pub fn retries(&self) -> u32 {
        self.0
            .properties
            .headers
            .as_ref()
            .map(|hdrs| match hdrs.get("retries") {
                Some(&AMQPValue::LongUInt(retries)) => retries,
                _ => 0,
            })
            .unwrap_or(0)
    }

    pub fn incr_retries(&mut self) -> u32 {
        let incrd_retries = self.retries() + 1;
        let mut headers = self.0
            .properties
            .headers
            .take()
            .unwrap_or_else(FieldTable::new);
        headers.insert("retries".to_string(), AMQPValue::LongUInt(incrd_retries));
        self.0.properties.headers = Some(headers);
        incrd_retries
    }

    pub fn should_retry(&mut self, max_retries: u32) -> bool {
        self.incr_retries() < max_retries
    }
}

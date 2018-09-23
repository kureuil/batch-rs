pub use failure::Error;
pub use futures::future::ok;
pub use futures::Future;
pub use futures::IntoFuture;
pub use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub use std::boxed::Box;
pub use std::marker::Send;
pub use std::result::Result;
pub use std::time::Duration;

#[allow(non_camel_case_types)]
pub type str = help::Str;

mod help {
	pub type Str = str;
}

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::message::Message;


#[derive(Clone, Serialize, Deserialize)]
pub struct UUID {
  pub uuid : [u8;16],
}
impl Message for UUID {}

// TODO: Consider replacing this with e.g. UUID from the crate "uuid"
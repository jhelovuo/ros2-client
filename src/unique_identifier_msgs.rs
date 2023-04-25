use serde::{Deserialize, Serialize};
use crate::message::Message;


#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct UUID {
  pub uuid : [u8;16],
}
impl Message for UUID {}

impl UUID {
  pub const ZERO: UUID = UUID{ uuid: [0;16] };
}

// TODO: Consider replacing this with e.g. UUID from the crate "uuid"
use serde::{Serialize, de::DeserializeOwned};

pub trait Message: Serialize + DeserializeOwned {}

impl Message for () {}
impl Message for String {}
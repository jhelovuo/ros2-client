use serde::{Serialize, de::DeserializeOwned};


/// Trait to ensure Messages can be (de)serialized
pub trait Message: Serialize + DeserializeOwned {}

impl Message for () {}
impl Message for String {}
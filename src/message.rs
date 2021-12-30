use serde::{Serialize, de::DeserializeOwned};

pub trait Message: Serialize + DeserializeOwned {
}
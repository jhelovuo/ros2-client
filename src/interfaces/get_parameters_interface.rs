use serde::{Deserialize, Serialize};

use crate::{parameters, Message, };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetParametersRequest {
  pub names: Vec<String>,
}
impl Message for GetParametersRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetParametersResponse {
  pub values: Vec<parameters::raw::ParameterValue>,
}
impl Message for GetParametersResponse {}

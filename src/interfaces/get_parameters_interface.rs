use serde::{Deserialize, Serialize};

use crate::{Message, Service, parameters};

pub struct GetParametersService {}

impl Service for GetParametersService {
  type Request = GetParametersRequest;
  type Response = GetParametersResponse;
  fn request_type_name(&self) -> &str {
    "rcl_interfaces::srv::dds_::GetParameters_Request_"
  }
  fn response_type_name(&self) -> &str {
    "rcl_interfaces::srv::dds_::GetParameters_Response_"
  }
}

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

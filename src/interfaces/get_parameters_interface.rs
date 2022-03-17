use rustdds::ros2::builtin_datatypes::ParameterValue;
use serde::{Deserialize, Serialize};

use crate::{Message, Service};

pub struct GetParametersService {}

impl Service for GetParametersService {
  type Request = GetParametersRequest;
  type Response = GetParametersResponse;
  fn request_type_name() -> String {
    "rcl_interfaces::srv::dds_::GetParameters_Request_".to_owned()
  }
  fn response_type_name() -> String {
    "rcl_interfaces::srv::dds_::GetParameters_Response_".to_owned()
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetParametersRequest {
  pub names: Vec<String>,
}
impl Message for GetParametersRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetParametersResponse {
  pub values: Vec<ParameterValue>,
}
impl Message for GetParametersResponse {}

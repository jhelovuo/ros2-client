use serde::{Deserialize, Serialize};

use crate::{parameters, service::AService, Message};

pub type ListParametersService = AService<ListParametersRequest, ListParametersResponse>;

pub type GetParametersService = AService<GetParametersRequest, GetParametersResponse>;

pub type GetParameterTypesService = AService<GetParameterTypesRequest, GetParameterTypesResponse>;

pub type SetParametersService = AService<SetParametersRequest, SetParametersResponse>;

// type DescribeParametersService = (); // TODO
// type GetParameterTypesService = (); // TODO
// type SetParametersAtomicallyService = (); // TODO

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListParametersRequest {
  pub prefixes: Vec<String>,
  pub depth: u64, //
}
impl Message for ListParametersRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListParametersResult {
  pub names: Vec<String>,
  pub prefixes: Vec<String>,
}
impl Message for ListParametersResult {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListParametersResponse {
  pub result: ListParametersResult,
}
impl Message for ListParametersResponse {}



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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetParameterTypesRequest {
  pub names: Vec<String>,
}
impl Message for GetParameterTypesRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetParameterTypesResponse {
  pub values: Vec<u8>,
}
impl Message for GetParameterTypesResponse {}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetParametersRequest {
  pub parameter: Vec<parameters::raw::Parameter>,
}
impl Message for SetParametersRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetParametersResponse {
  pub results: Vec<parameters::raw::SetParametersResult>,
}
impl Message for SetParametersResponse {}

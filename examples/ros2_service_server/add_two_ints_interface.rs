use ros2_client::{Message, Service};
use serde::{Deserialize, Serialize};

pub struct AddTwoIntsService {}

impl Service for AddTwoIntsService {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
    fn request_type_name() -> String {
        "example_interfaces::srv::dds_::AddTwoInts_Request_".to_owned()
    }
    fn response_type_name() -> String {
        "example_interfaces::srv::dds_::AddTwoInts_Response_".to_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTwoIntsRequest {
    pub a: i64,
    pub b: i64,
}
impl Message for AddTwoIntsRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTwoIntsResponse {
    pub sum: i64,
}
impl Message for AddTwoIntsResponse {}

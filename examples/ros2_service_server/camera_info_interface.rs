use ros2_client::{Message, Service};
use serde::{Deserialize, Serialize};

pub struct CameraInfoService {}

impl Service for CameraInfoService {
    type Request = CameraInfoRequest;
    type Response = CameraInfoResponse;
    fn request_type_name() -> String {
        "sensor_msgs::srv::dds_::SetCameraInfo_Request_".to_owned()
    }
    fn response_type_name() -> String {
        "sensor_msgs::srv::dds_::SetCameraInfo_Response_".to_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraInfoRequest {}
impl Message for CameraInfoRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraInfoResponse {
    pub success: bool,
    pub status_message: String,
}
impl Message for CameraInfoResponse {}

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::message::Message;

/// From https://docs.ros2.org/foxy/api/action_msgs/msg/GoalInfo.html
#[derive(Clone, Serialize, Deserialize)]
pub struct GoalInfo {
  goal_id : crate::unique_identifier_msgs::UUID,
  stamp: crate::builtin_interfaces::Time,
}
impl Message for GoalInfo {}



/// https://docs.ros2.org/foxy/api/action_msgs/msg/GoalStatus.html
#[derive(Clone, Serialize, Deserialize)]
pub struct GoalStatus {
  goal_info: GoalInfo,
  status: i8,
}
impl Message for GoalStatus {}

// TODO: Make this more Rust-like. E.g. an enum to avoid dealing with
// raw integers. And some serialization and deserialization.

impl GoalStatus {
  pub const UNKNOWN : i8 =  0 ;
  pub const ACCEPTED : i8 = 1 ;  
  pub const EXECUTING : i8 = 2 ;  
  pub const CANCELING : i8 = 3 ;  
  pub const SUCCEEDED : i8 = 4 ;  
  pub const CANCELED : i8 = 5 ;  
  pub const ABORTED : i8 = 6 ;  
}



/// https://docs.ros2.org/foxy/api/action_msgs/msg/GoalStatusArray.html
#[derive(Clone, Serialize, Deserialize)]
pub struct GoalStatusArray {
  status_list : Vec<GoalStatus>,
}
impl Message for GoalStatusArray {}




#[derive(Clone, Serialize, Deserialize)]
pub struct CancelGoalRequest {
  goal_info : GoalInfo,
}
impl Message for CancelGoalRequest {}




#[derive(Clone, Serialize, Deserialize)]
pub struct CancelGoalResponse {
  return_code: i8,
  goals_canceling: Vec<GoalInfo>  
}
impl Message for CancelGoalResponse {}

impl CancelGoalResponse {
  pub const ERROR_NONE: i8 = 0; // Indicates the request was accepted without any errors.
  pub const ERROR_REJECTED: i8 = 1; // 
  pub const ERROR_UNKNOWN_GOAL: i8 = 2;
  pub const ERROR_GOAL_TERMINATED: i8 = 3;
}
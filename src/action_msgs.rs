use serde::{Deserialize, Serialize};
use serde_repr::{Serialize_repr, Deserialize_repr};

use crate::message::Message;

/// From https://docs.ros2.org/foxy/api/action_msgs/msg/GoalInfo.html
#[derive(Clone, Serialize, Deserialize)]
pub struct GoalInfo {
  goal_id : crate::unique_identifier_msgs::UUID,
  stamp: crate::builtin_interfaces::Time,
}
impl Message for GoalInfo {}

#[derive(Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(i8)]
pub enum GoalStatusEnum {
  Unknown = 0,
  Accepted = 1,
  Executing = 2,
  Canceling = 3,
  Succeeded = 4,
  Canceled = 5,
  Aborted = 6,
}



/// https://docs.ros2.org/foxy/api/action_msgs/msg/GoalStatus.html
#[derive(Clone, Serialize, Deserialize)]
pub struct GoalStatus {
  goal_info: GoalInfo,
  status: GoalStatusEnum,
}
impl Message for GoalStatus {}


/// https://docs.ros2.org/foxy/api/action_msgs/msg/GoalStatusArray.html
#[derive(Clone, Serialize, Deserialize)]
pub struct GoalStatusArray {
  status_list : Vec<GoalStatus>,
}
impl Message for GoalStatusArray {}



///https://docs.ros2.org/foxy/api/action_msgs/srv/CancelGoal.htm
#[derive(Clone, Serialize, Deserialize)]
pub struct CancelGoalRequest {
  goal_info : GoalInfo,
}
impl Message for CancelGoalRequest {}

/// https://docs.ros2.org/foxy/api/action_msgs/srv/CancelGoal.htm
#[derive(Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(i8)]
pub enum CancelGoalResponseEnum {
  // Doc comments here copied from ROS2 message definition.

  /// Indicates the request was accepted without any errors. 
  /// One or more goals have transitioned to the CANCELING state. 
  /// The goals_canceling list is not empty.
  None = 0, 

  /// Indicates the request was rejected.
  /// No goals have transitioned to the CANCELING state. The goals_canceling list is
  /// empty.
  Rejected = 1,

  /// Indicates the requested goal ID does not exist.
  /// No goals have transitioned to the CANCELING state. The goals_canceling list is
  /// empty.
  UnknownGoal = 2,

  /// Indicates the goal is not cancelable because it is already in a terminal state.
  /// No goals have transitioned to the CANCELING state. The goals_canceling list is
  /// empty.
  GoalTerminated = 3,
}

/// https://docs.ros2.org/foxy/api/action_msgs/srv/CancelGoal.htm
#[derive(Clone, Serialize, Deserialize)]
pub struct CancelGoalResponse {
  return_code: CancelGoalResponseEnum,
  goals_canceling: Vec<GoalInfo>  
}
impl Message for CancelGoalResponse {}

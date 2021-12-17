use serde::{Deserialize, Serialize};

use rustdds::*;

/// ROS2 [ParameterEvent](https://github.com/ros2/rcl_interfaces/blob/master/rcl_interfaces/msg/ParameterEvent.msg)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterEvent {
  timestamp: Timestamp,
  // fully qualified path
  node: String,
  new_parameters: Vec<Parameter>,
  changed_parameters: Vec<Parameter>,
  deleted_parameters: Vec<Parameter>,
}

/// [Parameter](https://github.com/ros2/rcl_interfaces/blob/master/rcl_interfaces/msg/Parameter.msg)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
  name: String,
  value: ParameterValue,
}

/// [ParameterValue](https://github.com/ros2/rcl_interfaces/blob/master/rcl_interfaces/msg/ParameterValue.msg)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValue {
  ptype: u8,
  boolean_value: bool,
  int_value: i64,
  double_value: f64,
  string_value: String,
  byte_array: Vec<u8>,
  bool_array: Vec<bool>,
  int_array: Vec<i64>,
  double_array: Vec<f64>,
  string_array: Vec<String>,
}

/// ROS2 defines this as an empty .msg
/// [ParameterType](https://github.com/ros2/rcl_interfaces/blob/master/rcl_interfaces/msg/ParameterType.msg)
pub struct ParameterType {}

impl ParameterType {
  pub const NOT_SET:u8 = 0;

  pub const BOOL:u8 = 1;
  pub const INTEGER:u8 = 2;
  pub const DOUBLE:u8 = 3;
  pub const STRING:u8 = 4;
  pub const BYTE_ARRAY:u8 = 5;
  pub const BOOL_ARRAY:u8 = 6;
  pub const INTEGER_ARRAY:u8 = 7;
  pub const DOUBLE_ARRAY:u8 = 8;
  pub const STRING_ARRAY:u8 = 9;
}
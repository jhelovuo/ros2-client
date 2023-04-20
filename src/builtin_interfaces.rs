use serde::{Deserialize, Serialize};
use crate::message::Message;

#[derive(Clone, Serialize, Deserialize)]
pub struct Time {
  pub sec: i32,
  pub nanosec: u32,
}
impl Message for Time {}

// TODO: Implement constructors and conversions to/from usual Rust time formats
// Note that this type does not specifiy a zero point in time.

#[derive(Clone, Serialize, Deserialize)]
pub struct Duration {
  pub sec: i32, // ROS2: Seconds component, range is valid over any possible int32 value.
  pub nanosec: u32,// ROS2:  Nanoseconds component in the range of [0, 10e9).
  // TODO: How does the nanoseconds component work with negative seconds?
}
impl Message for Duration {}


// TODO: Implement the usual time arithmetic for Time and Duration, i.e.
// Time - Time = Duration
// Time + Duration = Time
// Time - Duration = time
// Duration + Duration = Duration
// Duration - Duration = Duration
// Implement a "zero" Duration value
// Implement conversions to/from Rust's usual Duration types.
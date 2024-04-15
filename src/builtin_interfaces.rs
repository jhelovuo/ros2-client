use serde::{Deserialize, Serialize};
use log::error;

use crate::{message::Message, ros_time::ROSTime};

// https://index.ros.org/p/builtin_interfaces/
//
// Defines message types Duration and Time .
//
// The name "builtin_interfaces" is not very descriptive, but that is how
// it is in ROS.

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Time {
  pub sec: i32,
  pub nanosec: u32,
}
impl Message for Time {}

impl Time {
  pub const ZERO: Time = Time { sec: 0, nanosec: 0 };

  pub const DUMMY: Time = Time {
    sec: 1234567890,
    nanosec: 1234567890,
  };

  /// Returns the current time for the system clock.
  ///
  /// To use simulation-capable time, ask from `Node`.
  pub(crate) fn now() -> Self {
    match chrono::Utc::now().timestamp_nanos_opt() {
      None => {
        error!("Timestamp out of range.");
        Time::ZERO // Since we have to return something
      }
      Some(negative) if negative < 0 => {
        error!("Timestamp out of range (negative).");
        Time::ZERO // Since we have to return something
      }
      Some(non_negative) => Self::from_nanos(non_negative),
    }
  }

  pub fn from_nanos(nanos_since_epoch: i64) -> Self {
    Self {
      sec: (nanos_since_epoch / 1_000_000_000) as i32,
      nanosec: (nanos_since_epoch % 1_000_000_000) as u32,
    }
  }

  pub fn to_nanos(&self) -> i64 {
    (self.sec as i64) * 1_000_000_000 + (self.nanosec as i64)
  }
}

// NOTE:
// This may panic, if the source ROSTime is unreasonably far in the past or
// future. If this is not ok, then TryFrom should be implemented and used.
impl From<ROSTime> for Time {
  fn from(rt: ROSTime) -> Time {
    Time::from_nanos(rt.to_nanos().unwrap())
  }
}

impl From<Time> for ROSTime {
  fn from(t: Time) -> ROSTime {
    ROSTime::from_nanos(t.to_nanos())
  }
}

// TODO: Implement constructors and conversions to/from usual Rust time formats
// Note that this type does not specify a zero point in time.

// Converting a straight 64-bit nanoseconds value to Duration is non-trivial.
// See function `Duration::operator builtin_interfaces::msg::Duration() const`
// in https://github.com/ros2/rclcpp/blob/rolling/rclcpp/src/rclcpp/duration.cpp
//
// If dividing the raw nanosecond duration by 10^9 would overflow `i32`, then
// saturate to either to {sec = i32::max , nanosec = u32::max} (positive
// overflow) or { sec = i32::min , nanosec = 0 }.
//
// Converting non-negative nanoseconds to Duration is straightforward. Just use
// integer division by 10^9 and store quotient and remainder.
//
// Negative nanoseconds are converted by similar integer divsion, and the result
// is { sec = quotient - 1 , nanosec = 10^9 + remainder}
//
// E.g. -1.5*10^9 nanosec --> quotient = -1 , remainder = -5*10^8
// (We are using division with invariant: quotient * divisor + remainder ==
// dividend ) Now { sec = -2 , nanosec = +5 * 10^8 }
//
// -1 nanosec --> quotient = 0, remainder = -1 -->
// { sec = -1 , nanosec = 999_999_999 }

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Duration {
  pub sec: i32, // ROS2: Seconds component, range is valid over any possible int32 value.
  pub nanosec: u32, /* ROS2:  Nanoseconds component in the range of [0, 10e9). */
}
impl Message for Duration {}

impl Duration {
  pub const fn zero() -> Self {
    Self { sec: 0, nanosec: 0 }
  }

  pub const fn from_secs(sec: i32) -> Self {
    Self { sec, nanosec: 0 }
  }

  pub const fn from_millis(millis: i64) -> Self {
    let nanos = millis * 1_000_000; // Maybe overflow, but result will also.
    Self::from_nanos(nanos)
  }

  pub const fn from_nanos(nanos: i64) -> Self {
    // This algorithm is from
    // https://github.com/ros2/rclcpp/blob/ea8daa37845e6137cba07a18eb653d97d87e6174/rclcpp/src/rclcpp/duration.cpp
    // lines 61-88

    // Except that we also test for quot underflow in case rem == 0

    let quot = nanos / 1_000_000_000;
    let rem = nanos % 1_000_000_000;
    // Rust `%` is the remainder operator.
    // If rem is negative, so is nanos
    if rem >= 0 {
      // positive or zero duration
      if quot > (i32::MAX as i64) {
        // overflow => saturate to max
        Duration {
          sec: i32::MAX,
          nanosec: u32::MAX,
        }
      } else if quot <= (i32::MIN as i64) {
        // underflow => saturate to min
        Duration {
          sec: i32::MIN,
          nanosec: 0,
        }
      } else {
        // normal case
        Duration {
          sec: quot as i32,
          nanosec: rem as u32,
        }
        // as-conversions will succeed: we know 0 <= quot <= i32::MAX, and
        // also 0 <= rem <= 1_000_000_000
      }
    } else {
      // duration was negative
      if quot <= (i32::MIN as i64) {
        // underflow => saturate to min
        Duration {
          sec: i32::MIN,
          nanosec: 0,
        }
      } else {
        // normal negative result
        Duration {
          sec: (quot + 1) as i32,
          nanosec: (1_000_000_000 + rem) as u32,
        }
        // i32::MIN <= quot < 0 => quot+1 is valid i32
        // -999_999_999 <= rem < 0 =>
        // 1 <= 1_000_000_000 + rem < 1_000_000_000 => valid u32
      }
    }
  }

  pub fn to_nanos(&self) -> i64 {
    let s = self.sec as i64;
    let ns = self.nanosec as i64;

    1_000_000_000 * s + ns
  }
}

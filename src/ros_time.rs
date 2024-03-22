use std::{
  convert::TryFrom,
  ops::{Add, Sub},
  time::Duration,
};

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use log::error;

/// ROS Time with nanosecond precision
///
/// This is the in-memory representation of builtin_interfaces::Time
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize)]
pub struct ROSTime {
  // really, this is just a wrpper over chrono::Utc.
  // But we could change the representaiton, if necessary
  chrono_time: DateTime<Utc>,
}

impl ROSTime {
  /// Returns the current time for the system clock.
  ///
  /// To use simulation-capable time, ask from `Node`.
  pub(crate) fn now() -> Self {
    Self {
      chrono_time: chrono::Utc::now(),
    }
  }

  pub const ZERO: Self = Self::from_nanos(0);

  /// Fallible conversion to nanoseconds since non-leap-nanoseconds since
  /// January 1, 1970 UTC
  ///
  /// chrono docs:
  ///
  /// An i64 with nanosecond precision can span a range of ~584 years. This
  /// function returns None on an out of range DateTime. The dates that can be
  /// represented as nanoseconds are between 1677-09-21T00:12:43.145224192 and
  /// 2262-04-11T23:47:16.854775807
  pub fn to_nanos(&self) -> Option<i64> {
    self.chrono_time.timestamp_nanos_opt()
  }

  /// Like `to_nanos()` , but yields ZERO timestmap if `i64` would be exceeded.
  pub fn to_nanos_or_zero(&self) -> i64 {
    match self.to_nanos() {
      None => {
        // Outf of range in either direction.
        error!("Timestamp out of range.");
        0 // Since we have to return something
      }
      Some(nanos) => nanos,
    }
  }

  pub const fn from_nanos(nanos_since_unix_epoch: i64) -> Self {
    Self {
      chrono_time: chrono::DateTime::<Utc>::from_timestamp_nanos(nanos_since_unix_epoch),
    }
  }
}

impl From<chrono::DateTime<Utc>> for ROSTime {
  fn from(chrono_time: chrono::DateTime<Utc>) -> ROSTime {
    ROSTime{ chrono_time }
  }
}

impl From<ROSTime> for chrono::DateTime<Utc> {
  fn from(rt: ROSTime) -> chrono::DateTime<Utc> {
    rt.chrono_time
  }
}


impl Sub for ROSTime {
  type Output = ROSDuration;

  fn sub(self, other: ROSTime) -> ROSDuration {
    ROSDuration {
      diff: self.chrono_time - other.chrono_time,
    }
  }
}

impl Sub<ROSDuration> for ROSTime {
  type Output = ROSTime;

  fn sub(self, other: ROSDuration) -> ROSTime {
    ROSTime {
      chrono_time: self.chrono_time - other.diff,
    }
  }
}

impl Add<ROSDuration> for ROSTime {
  type Output = ROSTime;

  fn add(self, other: ROSDuration) -> ROSTime {
    ROSTime {
      chrono_time: self.chrono_time + other.diff,
    }
  }
}

/// Difference between ROSTime or SystemTime instances
pub struct ROSDuration {
  diff: chrono::TimeDelta,
}

impl ROSDuration {
  /// Construct from nanosecond count
  pub const fn from_nanos(nanos: i64) -> Self {
    ROSDuration {
      diff: chrono::TimeDelta::nanoseconds(nanos),
    }
  }

  /// Convert to nanoseconds.
  /// Returns `None` if `i64` would overflow.
  pub const fn to_nanos(&self) -> Option<i64> {
    self.diff.num_nanoseconds()
  }
}

impl TryFrom<Duration> for ROSDuration {
  type Error = chrono::OutOfRangeError;
  fn try_from(std_duration: Duration) -> Result<Self, Self::Error> {
    Ok(ROSDuration {
      diff: chrono::Duration::from_std(std_duration)?,
    })
  }
}

impl TryFrom<ROSDuration> for Duration {
  type Error = chrono::OutOfRangeError;
  fn try_from(duration: ROSDuration) -> Result<Duration, Self::Error> {
    duration.diff.to_std()
  }
}

impl Add for ROSDuration {
  type Output = ROSDuration;
  fn add(self, other: ROSDuration) -> ROSDuration {
    ROSDuration {
      diff: self.diff + other.diff,
    }
  }
}

impl Sub for ROSDuration {
  type Output = ROSDuration;
  fn sub(self, other: ROSDuration) -> ROSDuration {
    ROSDuration {
      diff: self.diff - other.diff,
    }
  }
}

/// Same as ROSTime, except this one cannot be simulated.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize)]
pub struct SystemTime {
  ros_time: ROSTime,
}

//TODO: implementation missing
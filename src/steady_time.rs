// Steady time has a arbitrary origin, but is guaranteed to run
// monotonically, regardless of clock corrections or timezones.
//
// Steady time should be used onyl when it is necessary, e.g. because of interacting with hardware.
// Steady time cannot be simulated. Use ROS time instead, whenever possible.
//

use std::fmt;
use std::ops::{Add, Sub};
use std::convert::TryFrom;
use std::time::Duration;
use serde::{Serialize,Deserialize};

use chrono::{DateTime,Utc};
use libc;

// Monotonic time in nanoseconds
//H
// To get offset to UTC time, use now_with_utc() note that this will change over time, latest
// at the next leap second.
#[derive(Clone,Copy,PartialEq,PartialOrd,Eq,Ord, Debug, Serialize, Deserialize)]
pub struct Time {
  nano_counter : i64,  // free-running nanosecond counter
}

impl Time {
  pub fn now() -> Time {
    let mut ts = libc::timespec { tv_sec: 0 , tv_nsec: 0, };
    unsafe {
      assert_eq!(0, libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts));
    }
    Time{ nano_counter: (ts.tv_sec as i64) * (10i64.pow(9)) + (ts.tv_nsec as i64) }
  }

  pub fn now_with_utc() -> (Time,DateTime<Utc>) {
    let m0 = Self::now();
    let utc = Utc::now();
    let m1 = Self::now();
    let diff = m1 - m0;
    //println!("now_with_utc() diff = {} ns" , diff.as_nanos() );
    // TODO: check that diff is very small and complain if not

    // We add half of the diff to compensate for the difference in call times.
    ( m0 + TimeDiff::from_nanos( diff.as_nanos() / 2)
    , utc
    )
  }

  pub fn as_i64(self) -> i64 {
    self.nano_counter
  }

  pub fn from_i64(nanos :i64) -> Time {
    Time{ nano_counter: nanos }
  }

  // Time cannot implement the Zero trait, because Time does not implement Add (with itself).
  // So we just implement a zero() function.
  pub fn zero() -> Time
  {
    Time { nano_counter: 0 }
  }
}

impl fmt::Display for Time {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // TODO: needs a display customization
        fmt::Debug::fmt(self, fmt)
    }
}

impl Sub for Time {
    type Output = TimeDiff;

    fn sub(self, other: Time) -> TimeDiff {
        TimeDiff::from_nanos( self.nano_counter - other.nano_counter )
    }
}

impl Sub<TimeDiff> for Time {
    type Output = Time;

    fn sub(self, other: TimeDiff) -> Time {
        Time{ nano_counter: self.nano_counter - other.as_nanos() }
    }
}

impl Add<TimeDiff> for Time {
    type Output = Time;

    fn add(self, other: TimeDiff) -> Time {
        Time{ nano_counter: self.nano_counter + other.as_nanos() }
    }
}

// time difference as nanoseconds, can be negative, unlike std::time::Duration
#[derive(Clone,Copy,PartialEq,PartialOrd,Eq,Ord, Debug, Serialize, Deserialize)]
pub struct TimeDiff {
  nano_diff : i64,
}

impl TimeDiff {
  pub fn as_i64(self) -> i64 {
    self.nano_diff
  }

  pub fn from_i64(nanos :i64) -> TimeDiff {
    TimeDiff{ nano_diff: nanos }
  }

  pub const fn from_nanos(nanos : i64) -> TimeDiff {
    TimeDiff { nano_diff : nanos }
  }

  pub const fn from_millis(millis : i64) -> TimeDiff {
    TimeDiff { nano_diff : millis * 1_000_000 }
  }

  pub const fn from_secs(secs : i64) -> TimeDiff {
    TimeDiff { nano_diff : secs * 1_000_000_000 }
  }

  pub const fn as_nanos(self) -> i64 {
    self.nano_diff
  }

  pub const fn as_millis(self) -> i64 {
    self.nano_diff / 1_000_000
  }

  #[allow(dead_code)]
  pub const fn as_seconds(self) -> i64 {
    self.nano_diff / 1_000_000_000
  }

  pub /*const*/ fn as_duration(self) -> Result<Duration, <u64 as TryFrom<i64>>::Error> {
    u64::try_from(self.nano_diff).map( |n| Duration::from_nanos(n) )
  }

  pub /*const*/ fn as_saturating_duration(self) -> Duration {
    self.as_duration().unwrap_or( Duration::from_secs(0) )
  }

}

impl fmt::Display for TimeDiff {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // TODO: needs a display customization
        fmt::Debug::fmt(self, fmt)
    }
}

impl Add for TimeDiff {
  type Output = TimeDiff;
  fn add(self,other: TimeDiff) -> TimeDiff {
    TimeDiff{ nano_diff: self.nano_diff + other.nano_diff }
  }
}



impl Sub for TimeDiff {
  type Output = TimeDiff;
  fn sub(self,other: TimeDiff) -> TimeDiff {
    TimeDiff{ nano_diff: self.nano_diff - other.nano_diff }
  }
}
